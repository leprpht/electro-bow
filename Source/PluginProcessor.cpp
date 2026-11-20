#include "PluginProcessor.h"
#ifndef ELECTROBOW_TEST_BUILD
#include "PluginEditor.h"
#endif

#include <algorithm>
#include <cmath>

namespace {
constexpr int stateMagic = 0x45424F57; // EBOW
}

ElectroBowAudioProcessor::ElectroBowAudioProcessor()
    : AudioProcessor(BusesProperties()
                         .withInput("Input", juce::AudioChannelSet::stereo(), true)
                         .withOutput("Output", juce::AudioChannelSet::stereo(), true)) {
    auto add = [this](Control index, const char* id, const char* name, float maximum,
                      float initial) {
        auto attributes = juce::AudioParameterFloatAttributes();
        if (maximum > 1.0f)
            attributes = attributes.withLabel("ms");
        auto p = std::make_unique<juce::AudioParameterFloat>(
            juce::ParameterID{id, 1}, name, juce::NormalisableRange<float>{0.0f, maximum}, initial,
            attributes);
        controls[index] = p.get();
        addParameter(p.release()); // AudioProcessor owns the parameter.
    };
    add(pressure, "bowPressure", "Bow Pressure", 1.0f, 0.5f);
    add(speed, "bowSpeed", "Bow Speed", 1.0f, 0.5f);
    add(friction, "friction", "Bow Friction", 1.0f, 0.127f);
    add(attack, "attackMs", "Attack", 500.0f, 50.0f);
    add(resonance, "naturalResonanceMs", "Resonance", 1000.0f, 200.0f);
    add(character, "bowCharacter", "Bow Character", 1.0f, 0.45f);
    add(sustain, "sustain", "Sustain", 1.0f, 0.5f);
}

void ElectroBowAudioProcessor::prepareToPlay(double sampleRate, int samplesPerBlock) {
    filterBank.prepare(sampleRate, std::max(1, samplesPerBlock), getTotalNumInputChannels());
    wetBuffer.setSize(getTotalNumInputChannels(), filterBank.getCapacity());
    bypassMix.reset(sampleRate, 0.005);
    inputLevel.store(0.0f, std::memory_order_relaxed);
    firstBlock = true;
    setLatencySamples(0);
}

void ElectroBowAudioProcessor::releaseResources() {
    reset();
}

void ElectroBowAudioProcessor::reset() {
    inputLevel.store(0.0f, std::memory_order_relaxed);
    firstBlock = true;
    filterBank.reset();
}

bool ElectroBowAudioProcessor::isBusesLayoutSupported(const BusesLayout& layouts) const {
    const auto input = layouts.getMainInputChannelSet();

    const auto output = layouts.getMainOutputChannelSet();

    if (input != juce::AudioChannelSet::mono() && input != juce::AudioChannelSet::stereo()) {
        return false;
    }

    return output == input;
}

void ElectroBowAudioProcessor::processBlock(juce::AudioBuffer<float>& buffer,
                                            juce::MidiBuffer& midi) {
    juce::ignoreUnused(midi);
    render(buffer, false);
}

void ElectroBowAudioProcessor::processBlockBypassed(juce::AudioBuffer<float>& buffer,
                                                    juce::MidiBuffer& midi) {
    juce::ignoreUnused(midi);
    render(buffer, true);
}

void ElectroBowAudioProcessor::render(juce::AudioBuffer<float>& buffer, bool bypassed) {
    juce::ScopedNoDenormals noDenormals;
    const int channels = getTotalNumInputChannels();
    const int samples = buffer.getNumSamples();
    for (int channel = channels; channel < buffer.getNumChannels(); ++channel)
        buffer.clear(channel, 0, samples);

    if (filterBank.getCapacity() == 0 || filterBank.getNumChannels() != channels ||
        buffer.getNumChannels() < channels) {
        buffer.clear();
        return;
    }

    // Keep one control snapshot for the entire host callback, including chunks.
    if (samples == 0)
        return;
    const EbBowControls settings{controls[attack]->get(),   controls[character]->get(),
                                 controls[sustain]->get(),  controls[resonance]->get(),
                                 controls[pressure]->get(), controls[speed]->get(),
                                 controls[friction]->get()};
    const float dryTarget = bypassed ? 1.0f : 0.0f;
    if (firstBlock) {
        bypassMix.setCurrentAndTargetValue(dryTarget);
        firstBlock = false;
    }
    bypassMix.setTargetValue(dryTarget);

    // A host may exceed its advertised maximum. Chunk without reallocating or
    // dropping audio; each channel's filter history continues across chunks.
    for (int offset = 0; offset < samples;) {
        const int count = std::min(filterBank.getCapacity(), samples - offset);
        std::array<const float*, FilterBank::maxChannels> input{};
        std::array<float*, FilterBank::maxChannels> output{};
        const bool mixDry = bypassed || bypassMix.isSmoothing();
        for (int channel = 0; channel < channels; ++channel) {
            input[static_cast<std::size_t>(channel)] = buffer.getReadPointer(channel, offset);
            output[static_cast<std::size_t>(channel)] =
                mixDry ? wetBuffer.getWritePointer(channel)
                       : buffer.getWritePointer(channel, offset);
        }

        filterBank.process(input.data(), output.data(), count, settings);
        if (mixDry && bypassMix.isSmoothing()) {
            for (int i = 0; i < count; ++i) {
                const float dryGain = bypassMix.getNextValue();
                for (int ch = 0; ch < channels; ++ch) {
                    const float dry = input[static_cast<size_t>(ch)][i];
                    buffer.setSample(ch, offset + i,
                                     dryGain * (std::isfinite(dry) ? dry : 0.0f) +
                                         (1.0f - dryGain) * output[static_cast<size_t>(ch)][i]);
                }
            }
        }
        offset += count;
    }
    inputLevel.store(filterBank.getInputLevel(), std::memory_order_relaxed);
}

juce::AudioProcessorEditor* ElectroBowAudioProcessor::createEditor() {
#ifdef ELECTROBOW_TEST_BUILD
    return nullptr;
#else
    return new ElectroBowAudioProcessorEditor(*this);
#endif
}

bool ElectroBowAudioProcessor::hasEditor() const {
    return true;
}

const juce::String ElectroBowAudioProcessor::getName() const {
    return JucePlugin_Name;
}

bool ElectroBowAudioProcessor::acceptsMidi() const {
    return false;
}

bool ElectroBowAudioProcessor::producesMidi() const {
    return false;
}

bool ElectroBowAudioProcessor::isMidiEffect() const {
    return false;
}

double ElectroBowAudioProcessor::getTailLengthSeconds() const {
    // Allow 90 dB of decay at the longest body setting, including damping.
    return 1.6;
}

int ElectroBowAudioProcessor::getNumPrograms() {
    return 1;
}

int ElectroBowAudioProcessor::getCurrentProgram() {
    return 0;
}

void ElectroBowAudioProcessor::setCurrentProgram(int index) {
    juce::ignoreUnused(index);
}

const juce::String ElectroBowAudioProcessor::getProgramName(int index) {
    juce::ignoreUnused(index);

    return {};
}

void ElectroBowAudioProcessor::changeProgramName(int index, const juce::String& newName) {
    juce::ignoreUnused(index);
    juce::ignoreUnused(newName);
}

void ElectroBowAudioProcessor::getStateInformation(juce::MemoryBlock& destData) {
    juce::MemoryOutputStream stream(destData, false);

    stream.writeInt(stateMagic);
    stream.writeInt(3);
    for (const auto* parameter : controls)
        stream.writeFloat(parameter->get());
}

void ElectroBowAudioProcessor::setStateInformation(const void* data, int sizeInBytes) {
    if (data == nullptr || sizeInBytes < 8)
        return;
    juce::MemoryInputStream stream(data, static_cast<size_t>(sizeInBytes), false);
    if (stream.readInt() != stateMagic)
        return;
    const int version = stream.readInt();
    if (version < 1 || version > 3)
        return;
    const int count = version + 4;
    if (sizeInBytes < 8 + count * 4)
        return;

    // Legacy sessions did not store Sustain. v1 also predates Character.
    std::array<float, numControls> values{};
    for (int i = 0; i < count; ++i) {
        auto& value = values[static_cast<size_t>(i)];
        value = stream.readFloat();
        if (!std::isfinite(value))
            return;
    }
    for (size_t i = 0; i < controls.size(); ++i)
        controls[i]->setValueNotifyingHost(controls[i]->convertTo0to1(values[i]));
}

float ElectroBowAudioProcessor::getPitchFrequencyHz() const noexcept {
    return 0.0f;
}
float ElectroBowAudioProcessor::getPitchConfidence() const noexcept {
    return 0.0f;
}
int ElectroBowAudioProcessor::getPitchMidiNote() const noexcept {
    return 0;
}
int ElectroBowAudioProcessor::getActiveVoiceCount() const noexcept {
    return 0;
}
float ElectroBowAudioProcessor::getVoiceFrequencyHz(int index) const noexcept {
    juce::ignoreUnused(index);
    return 0.0f;
}
float ElectroBowAudioProcessor::getVoiceStrength(int index) const noexcept {
    juce::ignoreUnused(index);
    return 0.0f;
}

juce::var ElectroBowAudioProcessor::getUiState() const {
    auto* state = new juce::DynamicObject();
    auto* parameters = new juce::DynamicObject();

    for (const auto* parameter : controls)
        parameters->setProperty(parameter->paramID, parameter->get());
    state->setProperty("parameters", juce::var(parameters));
    state->setProperty("inputLevel", inputLevel.load(std::memory_order_relaxed));

    auto* pitch = new juce::DynamicObject();
    pitch->setProperty("note", juce::String("—"));
    pitch->setProperty("frequencyHz", 0.0f);
    pitch->setProperty("confidence", 0.0f);
    state->setProperty("pitch", juce::var(pitch));

    juce::Array<juce::var> voices;
    state->setProperty("voices", voices);

    return juce::var(state);
}

void ElectroBowAudioProcessor::setUiParameter(const juce::String& parameterId,
                                              float value) noexcept {
    if (!std::isfinite(value))
        return;

    for (size_t i = 0; i < controls.size(); ++i) {
        auto* parameter = controls[i];
        if (parameter->paramID != parameterId)
            continue;
        // Keyboard edits and double-click reset are single-value gestures.
        if (!uiGestures[i])
            parameter->beginChangeGesture();
        parameter->setValueNotifyingHost(parameter->convertTo0to1(value));
        if (!uiGestures[i])
            parameter->endChangeGesture();
        return;
    }
}

void ElectroBowAudioProcessor::setUiGesture(const juce::String& id, bool active) {
    for (size_t i = 0; i < controls.size(); ++i) {
        if (controls[i]->paramID != id || uiGestures[i] == active)
            continue;
        uiGestures[i] = active;
        if (active)
            controls[i]->beginChangeGesture();
        else
            controls[i]->endChangeGesture();
    }
}

void ElectroBowAudioProcessor::endUiGestures() {
    for (size_t i = 0; i < controls.size(); ++i)
        if (uiGestures[i])
            setUiGesture(controls[i]->paramID, false);
}

juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter() {
    return new ElectroBowAudioProcessor();
}

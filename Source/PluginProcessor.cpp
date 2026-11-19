#include "PluginProcessor.h"
#ifndef ELECTROBOW_TEST_BUILD
#include "PluginEditor.h"
#endif

#include <algorithm>
#include <cmath>

namespace {
constexpr const char* noteNames[] = {"C",  "C#", "D",  "D#", "E",  "F",
                                     "F#", "G",  "G#", "A",  "A#", "B"};
}

ElectroBowAudioProcessor::ElectroBowAudioProcessor()
    : AudioProcessor(BusesProperties()
                         .withInput("Input", juce::AudioChannelSet::stereo(), true)
                         .withOutput("Output", juce::AudioChannelSet::stereo(), true)) {}

void ElectroBowAudioProcessor::prepareToPlay(double sampleRate, int samplesPerBlock) {
    filterBank.prepare(sampleRate, std::max(1, samplesPerBlock), getTotalNumInputChannels());
    setLatencySamples(0);
}

void ElectroBowAudioProcessor::releaseResources() {
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
    juce::ScopedNoDenormals noDenormals;

    juce::ignoreUnused(midi);

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
    const auto attack = requestedAttackMs.load();
    const auto character = requestedBowCharacter.load();

    // A host may exceed its advertised maximum. Chunk without reallocating or
    // dropping audio; each channel's filter history continues across chunks.
    for (int offset = 0; offset < samples;) {
        const int count = std::min(filterBank.getCapacity(), samples - offset);
        std::array<const float*, FilterBank::maxChannels> input{};
        std::array<float*, FilterBank::maxChannels> output{};
        for (int channel = 0; channel < channels; ++channel) {
            input[static_cast<std::size_t>(channel)] = buffer.getReadPointer(channel, offset);
            output[static_cast<std::size_t>(channel)] = buffer.getWritePointer(channel, offset);
        }

        filterBank.process(input.data(), output.data(), count, attack, character);
        offset += count;
    }
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
    // Unequal band gains can expose short IIR decay tails. This is tail
    // allowance, not processing latency; the neutral sum still has no tail.
    return requestedAttackMs.load() > 0.0f || requestedBowCharacter.load() > 0.0f ? 0.1 : 0.0;
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

    constexpr int stateMagic = 0x45424F57; // "EBOW"

    constexpr int stateVersion = 2;

    stream.writeInt(stateMagic);
    stream.writeInt(stateVersion);

    stream.writeFloat(bowPressure);
    stream.writeFloat(bowSpeed);
    stream.writeFloat(friction);

    stream.writeFloat(attackMs);
    stream.writeFloat(naturalResonanceMs);
    stream.writeFloat(requestedBowCharacter.load());
}

void ElectroBowAudioProcessor::setStateInformation(const void* data, int sizeInBytes) {
    if (data == nullptr || sizeInBytes <= 0) {
        return;
    }

    juce::MemoryInputStream stream(data, static_cast<size_t>(sizeInBytes), false);

    constexpr int stateMagic = 0x45424F57;

    constexpr int stateVersion = 2;

    const int magic = stream.readInt();

    const int version = stream.readInt();

    if (magic != stateMagic || (version != 1 && version != stateVersion) ||
        sizeInBytes < (version == 1 ? 28 : 32)) {
        return;
    }

    // Validate the complete payload before publishing any controls. std::clamp
    // does not sanitize NaN; a non-finite Attack makes the Rust ABI reject audio.
    std::array<float, 6> values{};
    for (int i = 0; i < (version == 1 ? 5 : 6); ++i)
        values[static_cast<std::size_t>(i)] = stream.readFloat();
    if (!std::all_of(values.begin(), values.end(),
                     [](float value) { return std::isfinite(value); }))
        return;

    bowPressure = std::clamp(values[0], 0.0f, 1.0f);
    bowSpeed = std::clamp(values[1], 0.0f, 1.0f);
    friction = std::clamp(values[2], 0.0f, 1.0f);
    attackMs = std::clamp(values[3], 0.0f, 500.0f);
    naturalResonanceMs = std::max(0.0f, values[4]);
    requestedBowCharacter.store(std::clamp(values[5], 0.0f, 1.0f));
    requestedBowPressure.store(bowPressure);
    requestedBowSpeed.store(bowSpeed);
    requestedFriction.store(friction);
    requestedAttackMs.store(attackMs);
    requestedNaturalResonanceMs.store(naturalResonanceMs);
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

    parameters->setProperty("bowCharacter", requestedBowCharacter.load());
    parameters->setProperty("bowPressure", requestedBowPressure.load());
    parameters->setProperty("bowSpeed", requestedBowSpeed.load());
    parameters->setProperty("friction", requestedFriction.load());
    parameters->setProperty("attackMs", requestedAttackMs.load());
    parameters->setProperty("naturalResonanceMs", requestedNaturalResonanceMs.load());
    state->setProperty("parameters", juce::var(parameters));

    const float frequency = getPitchFrequencyHz();
    const int midiNote = getPitchMidiNote();
    const bool hasPitch = frequency > 0.0f && std::isfinite(frequency);

    auto* pitch = new juce::DynamicObject();
    pitch->setProperty("note", hasPitch ? juce::String(noteNames[midiNote % 12]) +
                                              juce::String((midiNote / 12) - 1)
                                        : juce::String("—"));
    pitch->setProperty("frequencyHz", hasPitch ? frequency : 0.0f);
    pitch->setProperty("confidence", hasPitch ? getPitchConfidence() : 0.0f);
    state->setProperty("pitch", juce::var(pitch));

    juce::Array<juce::var> voices;
    state->setProperty("voices", voices);

    return juce::var(state);
}

void ElectroBowAudioProcessor::setUiParameter(const juce::String& parameterId,
                                              float value) noexcept {
    if (!std::isfinite(value))
        return;

    if (parameterId == "bowCharacter") {
        requestedBowCharacter.store(std::clamp(value, 0.0f, 1.0f));
    } else if (parameterId == "bowPressure") {
        bowPressure = std::clamp(value, 0.0f, 1.0f);
        requestedBowPressure.store(bowPressure);
    } else if (parameterId == "bowSpeed") {
        bowSpeed = std::clamp(value, 0.0f, 1.0f);
        requestedBowSpeed.store(bowSpeed);
    } else if (parameterId == "friction") {
        friction = std::clamp(value, 0.0f, 1.0f);
        requestedFriction.store(friction);
    } else if (parameterId == "attackMs") {
        attackMs = std::clamp(value, 0.0f, 500.0f);
        requestedAttackMs.store(attackMs);
    } else if (parameterId == "naturalResonanceMs") {
        naturalResonanceMs = std::max(0.0f, value);
        requestedNaturalResonanceMs.store(naturalResonanceMs);
    }
}

juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter() {
    return new ElectroBowAudioProcessor();
}

#include "PluginProcessor.h"
#include "PluginEditor.h"

#include <algorithm>
#include <cmath>

ElectroBowAudioProcessor::ElectroBowAudioProcessor()
    : AudioProcessor(BusesProperties()
                         .withInput("Input", juce::AudioChannelSet::stereo(), true)
                         .withOutput("Output", juce::AudioChannelSet::stereo(), true)) {
    polyphonicAnalyzer.setPitchTracker(
        [this](int voiceId, const std::vector<float>& samples, double rate) {
            return trackIsolatedVoice(voiceId, samples, rate);
        });
}

PolyphonicAnalyzer::PitchEstimate
ElectroBowAudioProcessor::trackIsolatedVoice(int voiceId, const std::vector<float>& samples,
                                             double rate) {
    if (voiceId < 0 || samples.empty())
        return {};

    int slot = findTrackerSlot(voiceId);

    if (slot < 0)
        slot = findFreeTrackerSlot();

    if (slot < 0)
        return {};

    auto& tracker = isolatedTrackers[static_cast<std::size_t>(slot)];

    const std::size_t slotIndex = static_cast<std::size_t>(slot);

    /*
        If this is a new analyzer voice, reuse this
        PitchDetector slot and reset its history.
    */
    if (isolatedTrackerVoiceIds[slotIndex] != voiceId) {
        tracker.prepare(rate);

        isolatedTrackerVoiceIds[slotIndex] = voiceId;
    }

    tracker.push(samples.data(), static_cast<int>(samples.size()));

    const float frequency = tracker.getFrequencyHz();

    const float confidence = tracker.getConfidence();

    if (frequency <= 0.0f || !std::isfinite(frequency) || confidence <= 0.0f ||
        !std::isfinite(confidence)) {
        return {};
    }

    return {frequency, std::clamp(confidence, 0.0f, 1.0f)};
}

int ElectroBowAudioProcessor::findTrackerSlot(int voiceId) const noexcept {
    for (int i = 0; i < static_cast<int>(isolatedTrackerVoiceIds.size()); ++i) {
        if (isolatedTrackerVoiceIds[static_cast<std::size_t>(i)] == voiceId) {
            return i;
        }
    }

    return -1;
}

int ElectroBowAudioProcessor::findFreeTrackerSlot() const noexcept {
    for (int i = 0; i < static_cast<int>(isolatedTrackerVoiceIds.size()); ++i) {
        if (isolatedTrackerVoiceIds[static_cast<std::size_t>(i)] < 0) {
            return i;
        }
    }

    return -1;
}

void ElectroBowAudioProcessor::prepareToPlay(double sampleRate, int samplesPerBlock) {
    const int safeBlockSize = juce::jmax(1, samplesPerBlock);

    monoScratch.assign(static_cast<std::size_t>(safeBlockSize), 0.0f);

    polyphonicAnalyzer.prepare(sampleRate);

    bowTrigger.prepare(sampleRate);
    pendingNoteOn = false;
    lastAnalyzerGeneration = 0;

    isolatedTrackerVoiceIds.fill(-1);

    for (auto& tracker : isolatedTrackers)
        tracker.prepare(sampleRate);

    voiceManager.prepare(sampleRate, attackMs, naturalResonanceMs);

    voiceManager.setBowParameters(bowPressure, bowSpeed, friction);

    voiceManager.setEnvelopeParameters(attackMs, naturalResonanceMs);
}

void ElectroBowAudioProcessor::releaseResources() {
    voiceManager.reset();

    bowTrigger.reset();
    pendingNoteOn = false;
    lastAnalyzerGeneration = 0;

    isolatedTrackerVoiceIds.fill(-1);
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

    const int numChannels = buffer.getNumChannels();

    const int numSamples = buffer.getNumSamples();

    if (numChannels <= 0 || numSamples <= 0) {
        return;
    }

    /*
        The host normally respects the maximum block
        size supplied during prepareToPlay().

        Do not resize the scratch buffer on the audio
        thread.
    */
    if (static_cast<std::size_t>(numSamples) > monoScratch.size()) {
        buffer.clear();
        return;
    }

    /*
        Convert input to mono.

        PolyphonicAnalyzer receives one analysis signal.
    */
    std::fill(monoScratch.begin(), monoScratch.begin() + numSamples, 0.0f);

    for (int channel = 0; channel < numChannels; ++channel) {
        const float* input = buffer.getReadPointer(channel);

        for (int sample = 0; sample < numSamples; ++sample) {
            monoScratch[static_cast<std::size_t>(sample)] += input[sample];
        }
    }

    const float channelScale = 1.0f / static_cast<float>(numChannels);

    for (int sample = 0; sample < numSamples; ++sample) {
        monoScratch[static_cast<std::size_t>(sample)] *= channelScale;
    }

    for (int sample = 0; sample < numSamples; ++sample) {
        if (bowTrigger.processSample(monoScratch[static_cast<std::size_t>(sample)])) {
            pendingNoteOn = true;
        }
    }

    /*
        Analyze and separate the incoming audio.
    */
    polyphonicAnalyzer.push(monoScratch.data(), numSamples);

    /*
        VoiceManager decides which voices should start,
        continue, or enter natural resonance.
    */
    const auto analysisGeneration = polyphonicAnalyzer.getAnalysisGeneration();

    const bool hasFreshAnalysis = analysisGeneration != lastAnalyzerGeneration;

    const bool allowNewVoices =
        pendingNoteOn || (voiceManager.getActiveVoiceCount() == 0 && bowTrigger.isNoteActive());

    voiceManager.updateDetectedVoices(polyphonicAnalyzer, allowNewVoices);

    // Consume the physical attack once an analyzed voice set has arrived.
    // All candidates in that frame may start, while later harmonic/ID
    // changes from the same pluck cannot create another attack.
    if (hasFreshAnalysis && pendingNoteOn && !polyphonicAnalyzer.getVoices().empty()) {
        pendingNoteOn = false;
    }

    lastAnalyzerGeneration = analysisGeneration;

    /*
        Render all active Bowed voices.
    */
    for (int sample = 0; sample < numSamples; ++sample) {
        const float output = voiceManager.processSample();

        for (int channel = 0; channel < numChannels; ++channel) {
            buffer.setSample(channel, sample, output);
        }
    }
}

juce::AudioProcessorEditor* ElectroBowAudioProcessor::createEditor() {
    return new ElectroBowAudioProcessorEditor(*this);
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
    return static_cast<double>(naturalResonanceMs) * 0.001 + 0.25;
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

    constexpr int stateVersion = 1;

    stream.writeInt(stateMagic);
    stream.writeInt(stateVersion);

    stream.writeFloat(bowPressure);
    stream.writeFloat(bowSpeed);
    stream.writeFloat(friction);

    stream.writeFloat(attackMs);
    stream.writeFloat(naturalResonanceMs);
}

void ElectroBowAudioProcessor::setStateInformation(const void* data, int sizeInBytes) {
    if (data == nullptr || sizeInBytes <= 0) {
        return;
    }

    juce::MemoryInputStream stream(data, static_cast<size_t>(sizeInBytes), false);

    constexpr int stateMagic = 0x45424F57;

    constexpr int stateVersion = 1;

    const int magic = stream.readInt();

    const int version = stream.readInt();

    if (magic != stateMagic || version != stateVersion) {
        return;
    }

    bowPressure = std::clamp(stream.readFloat(), 0.0f, 1.0f);

    bowSpeed = std::clamp(stream.readFloat(), 0.0f, 1.0f);

    friction = std::clamp(stream.readFloat(), 0.0f, 1.0f);

    attackMs = std::max(0.0f, stream.readFloat());

    naturalResonanceMs = std::max(0.0f, stream.readFloat());

    voiceManager.setBowParameters(bowPressure, bowSpeed, friction);

    voiceManager.setEnvelopeParameters(attackMs, naturalResonanceMs);
}

float ElectroBowAudioProcessor::getPitchFrequencyHz() const noexcept {
    for (int i = 0; i < VoiceManager::kMaxVoices; ++i) {
        if (voiceManager.isVoiceActive(i)) {
            return voiceManager.getVoiceFrequencyHz(i);
        }
    }

    return 0.0f;
}

float ElectroBowAudioProcessor::getPitchConfidence() const noexcept {
    for (int i = 0; i < VoiceManager::kMaxVoices; ++i) {
        if (voiceManager.isVoiceActive(i)) {
            return voiceManager.getVoiceStrength(i);
        }
    }

    return 0.0f;
}

int ElectroBowAudioProcessor::getPitchMidiNote() const noexcept {
    const float frequency = getPitchFrequencyHz();

    if (frequency <= 0.0f || !std::isfinite(frequency)) {
        return 0;
    }

    const float midi = 69.0f + 12.0f * std::log2(frequency / 440.0f);

    const int note = static_cast<int>(std::lround(midi));

    return std::clamp(note, 0, 127);
}

int ElectroBowAudioProcessor::getActiveVoiceCount() const noexcept {
    return voiceManager.getActiveVoiceCount();
}

float ElectroBowAudioProcessor::getVoiceFrequencyHz(int index) const noexcept {
    return voiceManager.getVoiceFrequencyHz(index);
}

float ElectroBowAudioProcessor::getVoiceStrength(int index) const noexcept {
    return voiceManager.getVoiceStrength(index);
}

juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter() {
    return new ElectroBowAudioProcessor();
}

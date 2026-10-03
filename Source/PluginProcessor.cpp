#include "PluginProcessor.h"
#include "PluginEditor.h"

#include <algorithm>
#include <cmath>

namespace {
constexpr const char* noteNames[] = {"C",  "C#", "D",  "D#", "E",  "F",
                                     "F#", "G",  "G#", "A",  "A#", "B"};
}

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
    dynamicsScratch.assign(static_cast<std::size_t>(safeBlockSize), 0.0f);

    polyphonicAnalyzer.prepare(sampleRate);

    bowTrigger.prepare(sampleRate);
    inputDynamics.prepare(sampleRate);
    pendingNoteOn = false;
    lastAnalyzerGeneration = 0;

    isolatedTrackerVoiceIds.fill(-1);

    for (auto& tracker : isolatedTrackers)
        tracker.prepare(sampleRate);

    bowPressure = std::clamp(requestedBowPressure.load(), 0.0f, 1.0f);
    bowSpeed = std::clamp(requestedBowSpeed.load(), 0.0f, 1.0f);
    friction = std::clamp(requestedFriction.load(), 0.0f, 1.0f);
    attackMs = std::max(0.0f, requestedAttackMs.load());
    naturalResonanceMs = std::max(0.0f, requestedNaturalResonanceMs.load());

    audioBowPressure = bowPressure;
    audioBowSpeed = bowSpeed;
    audioFriction = friction;
    audioAttackMs = attackMs;
    audioNaturalResonanceMs = naturalResonanceMs;

    voiceManager.prepare(sampleRate, attackMs, naturalResonanceMs);

    voiceManager.setBowParameters(bowPressure, bowSpeed, friction);

    voiceManager.setEnvelopeParameters(attackMs, naturalResonanceMs);
}

void ElectroBowAudioProcessor::releaseResources() {
    voiceManager.reset();

    bowTrigger.reset();
    inputDynamics.reset();
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

    const float nextBowPressure = std::clamp(requestedBowPressure.load(), 0.0f, 1.0f);
    const float nextBowSpeed = std::clamp(requestedBowSpeed.load(), 0.0f, 1.0f);
    const float nextFriction = std::clamp(requestedFriction.load(), 0.0f, 1.0f);
    const float nextAttackMs = std::max(0.0f, requestedAttackMs.load());
    const float nextNaturalResonanceMs = std::max(0.0f, requestedNaturalResonanceMs.load());

    if (std::abs(nextBowPressure - audioBowPressure) > 0.000001f ||
        std::abs(nextBowSpeed - audioBowSpeed) > 0.000001f ||
        std::abs(nextFriction - audioFriction) > 0.000001f) {
        voiceManager.setBowParameters(nextBowPressure, nextBowSpeed, nextFriction);
        audioBowPressure = nextBowPressure;
        audioBowSpeed = nextBowSpeed;
        audioFriction = nextFriction;
    }

    if (std::abs(nextAttackMs - audioAttackMs) > 0.000001f ||
        std::abs(nextNaturalResonanceMs - audioNaturalResonanceMs) > 0.000001f) {
        voiceManager.setEnvelopeParameters(nextAttackMs, nextNaturalResonanceMs);
        audioAttackMs = nextAttackMs;
        audioNaturalResonanceMs = nextNaturalResonanceMs;
    }

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
        const float inputSample = monoScratch[static_cast<std::size_t>(sample)];

        // Capture only the rising physical attack. The analyzer may publish
        // several different spectral descriptions of the same pluck later.
        const bool triggered = bowTrigger.processSample(inputSample);

        dynamicsScratch[static_cast<std::size_t>(sample)] =
            inputDynamics.processSample(inputSample);

        if (triggered) {
            pendingNoteOn = true;
        }

        if (bowTrigger.consumeRelease()) {
            voiceManager.releaseAll();
            pendingNoteOn = false;
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

    // A high envelope is not a new attack. In particular, do not reopen
    // note-on eligibility merely because analyzer loss has released every
    // voice while the tail of the same pluck is still above the threshold.
    // New voices are admitted only by the rising-edge event captured in
    // pendingNoteOn.
    const bool allowNewVoices = pendingNoteOn;

    voiceManager.setInputLevel(inputDynamics.getLevel());

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
        // Apply the continuous musical envelope immediately before rendering
        // each sample. VoiceManager combines this global input intensity with
        // each voice's own spectral strength and envelope state.
        voiceManager.setInputLevel(dynamicsScratch[static_cast<std::size_t>(sample)]);
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

    requestedBowPressure.store(bowPressure);
    requestedBowSpeed.store(bowSpeed);
    requestedFriction.store(friction);
    requestedAttackMs.store(attackMs);
    requestedNaturalResonanceMs.store(naturalResonanceMs);
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

juce::var ElectroBowAudioProcessor::getUiState() const {
    auto* state = new juce::DynamicObject();
    auto* parameters = new juce::DynamicObject();

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
    for (int i = 0; i < VoiceManager::kMaxVoices; ++i) {
        if (!voiceManager.isVoiceActive(i))
            continue;

        auto* voice = new juce::DynamicObject();
        voice->setProperty("frequencyHz", getVoiceFrequencyHz(i));
        voice->setProperty("strength", getVoiceStrength(i));
        voices.add(juce::var(voice));
    }
    state->setProperty("voices", voices);

    return juce::var(state);
}

void ElectroBowAudioProcessor::setUiParameter(const juce::String& parameterId,
                                              float value) noexcept {
    if (!std::isfinite(value))
        return;

    if (parameterId == "bowPressure") {
        bowPressure = std::clamp(value, 0.0f, 1.0f);
        requestedBowPressure.store(bowPressure);
    } else if (parameterId == "bowSpeed") {
        bowSpeed = std::clamp(value, 0.0f, 1.0f);
        requestedBowSpeed.store(bowSpeed);
    } else if (parameterId == "friction") {
        friction = std::clamp(value, 0.0f, 1.0f);
        requestedFriction.store(friction);
    } else if (parameterId == "attackMs") {
        attackMs = std::max(0.0f, value);
        requestedAttackMs.store(attackMs);
    } else if (parameterId == "naturalResonanceMs") {
        naturalResonanceMs = std::max(0.0f, value);
        requestedNaturalResonanceMs.store(naturalResonanceMs);
    }
}

juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter() {
    return new ElectroBowAudioProcessor();
}

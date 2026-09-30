#include "PluginProcessor.h"
#include "PluginEditor.h"

ElectroBowAudioProcessor::ElectroBowAudioProcessor()
    : AudioProcessor(BusesProperties()
                         .withInput("Input", juce::AudioChannelSet::stereo(), true)
                         .withOutput("Output", juce::AudioChannelSet::stereo(), true)) {
    // The analyzer remains independent of any particular tracker. The current
    // product path uses the existing detector as an adapter for each isolated
    // channel; this can later be replaced by Q without changing separation or
    // VoiceManager.
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

    int slot = -1;
    for (int i = 0; i < static_cast<int>(isolatedTrackerVoiceIds.size()); ++i) {
        if (isolatedTrackerVoiceIds[static_cast<size_t>(i)] == voiceId) {
            slot = i;
            break;
        }
    }

    if (slot < 0) {
        for (int i = 0; i < static_cast<int>(isolatedTrackerVoiceIds.size()); ++i) {
            if (isolatedTrackerVoiceIds[static_cast<size_t>(i)] < 0) {
                slot = i;
                break;
            }
        }
    }

    // There can be at most kMaxVoices active IDs. If an old ID has not yet
    // been retired by the analyzer, reuse its slot rather than aliasing two
    // active IDs with modulo arithmetic.
    if (slot < 0)
        slot = voiceId % static_cast<int>(isolatedTrackers.size());

    auto& tracker = isolatedTrackers[static_cast<size_t>(slot)];
    if (isolatedTrackerVoiceIds[static_cast<size_t>(slot)] != voiceId) {
        tracker.prepare(rate);
        isolatedTrackerVoiceIds[static_cast<size_t>(slot)] = voiceId;
    }

    tracker.push(samples.data(), static_cast<int>(samples.size()));
    if (tracker.getBestFrequencyHz() > 0.0f) {
        return {tracker.getBestFrequencyHz(), tracker.getBestConfidence()};
    }

    if (tracker.getNumNotes() <= 0)
        return {};

    const auto note = tracker.getNote(0);
    return {440.0f * std::pow(2.0f, static_cast<float>(note.midiNote - 69) / 12.0f),
            std::clamp(note.strength, 0.0f, 1.0f)};
}

void ElectroBowAudioProcessor::prepareToPlay(double sampleRate, int samplesPerBlock) {
    monoScratch.assign(static_cast<size_t>(juce::jmax(1, samplesPerBlock)), 0.0f);

    polyphonicAnalyzer.prepare(sampleRate);
    polyPitchDetector.prepare(sampleRate);
    isolatedTrackerVoiceIds.fill(-1);
    for (auto& tracker : isolatedTrackers)
        tracker.prepare(sampleRate);

    voiceManager.prepare(sampleRate, attackMs, releaseMs);

    voiceManager.setBowParameters(bowPressure, bowSpeed, friction);

    voiceManager.setEnvelopeParameters(attackMs, releaseMs);
}

void ElectroBowAudioProcessor::releaseResources() {
    voiceManager.reset();
    isolatedTrackerVoiceIds.fill(-1);
}

bool ElectroBowAudioProcessor::isBusesLayoutSupported(const BusesLayout& layouts) const {
    const auto& input = layouts.getMainInputChannelSet();

    const auto& output = layouts.getMainOutputChannelSet();

    if (input != output)
        return false;

    return input == juce::AudioChannelSet::mono() || input == juce::AudioChannelSet::stereo();
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

    if (monoScratch.size() < static_cast<size_t>(numSamples)) {
        monoScratch.resize(static_cast<size_t>(numSamples));
    }

    std::fill(monoScratch.begin(), monoScratch.begin() + numSamples, 0.0f);

    // ------------------------------------------------------------------
    // Stereo -> mono.
    // ------------------------------------------------------------------

    for (int channel = 0; channel < numChannels; ++channel) {
        const float* input = buffer.getReadPointer(channel);

        for (int sample = 0; sample < numSamples; ++sample) {
            monoScratch[static_cast<size_t>(sample)] += input[sample];
        }
    }

    const float channelScale = 1.0f / static_cast<float>(numChannels);

    for (int sample = 0; sample < numSamples; ++sample) {
        monoScratch[static_cast<size_t>(sample)] *= channelScale;
    }

    // ------------------------------------------------------------------
    // Detect the current polyphonic note set.
    // ------------------------------------------------------------------

    // Keep the old detector fed for the existing editor/comparison accessors.
    // It is not used as the analyzer's architectural foundation.
    polyPitchDetector.push(monoScratch.data(), numSamples);
    polyphonicAnalyzer.push(monoScratch.data(), numSamples);

    // ------------------------------------------------------------------
    // Synchronise VoiceManager with the detected note set.
    //
    // Existing notes remain alive.
    // Missing notes enter release.
    // New notes create new Bowed voices.
    // ------------------------------------------------------------------

    voiceManager.updateDetectedVoices(polyphonicAnalyzer);

    // ------------------------------------------------------------------
    // Render all active voices.
    // ------------------------------------------------------------------

    for (int sample = 0; sample < numSamples; ++sample) {
        const float output = voiceManager.processSample();

        for (int channel = 0; channel < numChannels; ++channel) {
            buffer.setSample(channel, sample, output);
        }
    }
}

bool ElectroBowAudioProcessor::hasEditor() const {
    return true;
}

juce::AudioProcessorEditor* ElectroBowAudioProcessor::createEditor() {
    return new ElectroBowAudioProcessorEditor(*this);
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
    return 2.0;
}

int ElectroBowAudioProcessor::getNumPrograms() {
    return 1;
}

int ElectroBowAudioProcessor::getCurrentProgram() {
    return 0;
}

void ElectroBowAudioProcessor::setCurrentProgram(int) {}

const juce::String ElectroBowAudioProcessor::getProgramName(int) {
    return {};
}

void ElectroBowAudioProcessor::changeProgramName(int, const juce::String&) {}

void ElectroBowAudioProcessor::getStateInformation(juce::MemoryBlock& destData) {
    destData.reset();
}

void ElectroBowAudioProcessor::setStateInformation(const void*, int) {}

juce::AudioProcessor* JUCE_CALLTYPE createPluginFilter() {
    return new ElectroBowAudioProcessor();
}

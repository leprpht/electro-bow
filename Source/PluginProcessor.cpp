#include "PluginProcessor.h"
#include "PluginEditor.h"

ElectroBowAudioProcessor::ElectroBowAudioProcessor()
    : AudioProcessor(BusesProperties()
                         .withInput("Input", juce::AudioChannelSet::stereo(), true)
                         .withOutput("Output", juce::AudioChannelSet::stereo(), true)) {}

void ElectroBowAudioProcessor::prepareToPlay(double sampleRate, int samplesPerBlock) {
    monoScratch.assign(static_cast<size_t>(juce::jmax(1, samplesPerBlock)), 0.0f);

    polyPitchDetector.prepare(sampleRate);

    voiceManager.prepare(sampleRate, attackMs, releaseMs);

    voiceManager.setBowParameters(bowPressure, bowSpeed, friction);

    voiceManager.setEnvelopeParameters(attackMs, releaseMs);
}

void ElectroBowAudioProcessor::releaseResources() {
    voiceManager.reset();
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

    polyPitchDetector.push(monoScratch.data(), numSamples);

    // ------------------------------------------------------------------
    // Synchronise VoiceManager with the detected note set.
    //
    // Existing notes remain alive.
    // Missing notes enter release.
    // New notes create new Bowed voices.
    // ------------------------------------------------------------------

    voiceManager.updateDetectedNotes(polyPitchDetector);

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
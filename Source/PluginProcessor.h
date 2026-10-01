#pragma once

#include <juce_audio_utils/juce_audio_utils.h>

#include "PitchDetector.h"
#include "PolyphonicAnalyzer.h"
#include "VoiceManager.h"

#include <array>
#include <vector>

class ElectroBowAudioProcessor : public juce::AudioProcessor
{
public:
    ElectroBowAudioProcessor();
    ~ElectroBowAudioProcessor() override = default;

    void prepareToPlay(
        double sampleRate,
        int samplesPerBlock) override;

    void releaseResources() override;

    bool isBusesLayoutSupported(
        const BusesLayout& layouts) const override;

    void processBlock(
        juce::AudioBuffer<float>&,
        juce::MidiBuffer&) override;

    juce::AudioProcessorEditor* createEditor() override;

    bool hasEditor() const override;

    const juce::String getName() const override;

    bool acceptsMidi() const override;

    bool producesMidi() const override;

    bool isMidiEffect() const override;

    double getTailLengthSeconds() const override;

    int getNumPrograms() override;

    int getCurrentProgram() override;

    void setCurrentProgram(int index) override;

    const juce::String getProgramName(
        int index) override;

    void changeProgramName(
        int index,
        const juce::String& newName) override;

    void getStateInformation(
        juce::MemoryBlock& destData) override;

    void setStateInformation(
        const void* data,
        int sizeInBytes) override;

    /*
        Debug / UI access.

        These values are derived from VoiceManager.
        Continuous frequency in Hz remains the actual
        pitch representation used by the synthesis.
    */

    float getPitchFrequencyHz() const noexcept;

    float getPitchConfidence() const noexcept;

    int getPitchMidiNote() const noexcept;

    int getActiveVoiceCount() const noexcept;

    float getVoiceFrequencyHz(int index) const noexcept;

    float getVoiceStrength(int index) const noexcept;

private:
    PolyphonicAnalyzer::PitchEstimate
    trackIsolatedVoice(
        int voiceId,
        const std::vector<float>& samples,
        double sampleRate);

    int findTrackerSlot(
        int voiceId) const noexcept;

    int findFreeTrackerSlot() const noexcept;

    /*
        Polyphonic spectral analyzer.

        It separates the incoming audio into approximate
        individual voices and maintains persistent IDs.
    */
    PolyphonicAnalyzer polyphonicAnalyzer;

    /*
        One monophonic PitchDetector per analyzer voice.

        These operate on the separated voice signals,
        rather than on the complete chord.
    */
    std::array<
        PitchDetector,
        PolyphonicAnalyzer::kMaxVoices
    > isolatedTrackers;

    /*
        Maps analyzer voice IDs to PitchDetector slots.
    */
    std::array<
        int,
        PolyphonicAnalyzer::kMaxVoices
    > isolatedTrackerVoiceIds{};

    /*
        Owns the actual STK Bowed instances and their
        per-voice envelopes.
    */
    VoiceManager voiceManager;

    /*
        Reused every processBlock.

        No resize is performed during normal audio
        processing.
    */
    std::vector<float> monoScratch;

    /*
        Current instrument parameters.
    */
    float bowPressure = 0.5f;
    float bowSpeed = 0.5f;
    float friction = 0.127f;

    float attackMs = 50.0f;
    float naturalResonanceMs = 200.0f;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(
        ElectroBowAudioProcessor)
};
#pragma once

#include <juce_audio_utils/juce_audio_utils.h>

#include "FilterBank.h"

#include <atomic>

class ElectroBowAudioProcessor : public juce::AudioProcessor {
  public:
    ElectroBowAudioProcessor();
    ~ElectroBowAudioProcessor() override = default;

    void prepareToPlay(double sampleRate, int samplesPerBlock) override;

    void releaseResources() override;

    bool isBusesLayoutSupported(const BusesLayout& layouts) const override;

    void processBlock(juce::AudioBuffer<float>&, juce::MidiBuffer&) override;

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

    const juce::String getProgramName(int index) override;

    void changeProgramName(int index, const juce::String& newName) override;

    void getStateInformation(juce::MemoryBlock& destData) override;

    void setStateInformation(const void* data, int sizeInBytes) override;

    // Legacy UI telemetry stays idle during the transparent filter-bank phase.
    float getPitchFrequencyHz() const noexcept;

    float getPitchConfidence() const noexcept;

    int getPitchMidiNote() const noexcept;

    int getActiveVoiceCount() const noexcept;

    float getVoiceFrequencyHz(int index) const noexcept;

    float getVoiceStrength(int index) const noexcept;

    // State exposed to the existing WebView UI; controls are inactive in Phase 1.
    juce::var getUiState() const;

    void setUiParameter(const juce::String& parameterId, float value) noexcept;

  private:
    FilterBank filterBank;

    /*
        Retained for session/UI compatibility; inactive during Phase 1.
    */
    float bowPressure = 0.5f;
    float bowSpeed = 0.5f;
    float friction = 0.127f;

    float attackMs = 50.0f;
    float naturalResonanceMs = 200.0f;

    std::atomic<float> requestedBowPressure{0.5f};
    std::atomic<float> requestedBowSpeed{0.5f};
    std::atomic<float> requestedFriction{0.127f};
    std::atomic<float> requestedAttackMs{50.0f};
    std::atomic<float> requestedNaturalResonanceMs{200.0f};

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(ElectroBowAudioProcessor)
};

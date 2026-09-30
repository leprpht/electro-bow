#pragma once

#include <juce_audio_utils/juce_audio_utils.h>

#include "PolyPitchDetector.h"
#include "VoiceManager.h"

#include <cmath>
#include <vector>

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

    // ------------------------------------------------------------------
    // Temporary compatibility functions for the editor.
    // They expose the strongest detected note.
    // ------------------------------------------------------------------

    float getPitchFrequencyHz() const noexcept {
        if (polyPitchDetector.getNumNotes() <= 0)
            return 0.0f;

        const auto note = polyPitchDetector.getNote(0);

        return 440.0f * std::pow(2.0f, static_cast<float>(note.midiNote - 69) / 12.0f);
    }

    float getPitchConfidence() const noexcept {
        if (polyPitchDetector.getNumNotes() <= 0)
            return 0.0f;

        return polyPitchDetector.getNote(0).strength;
    }

    int getPitchMidiNote() const noexcept {
        if (polyPitchDetector.getNumNotes() <= 0)
            return 0;

        return polyPitchDetector.getNote(0).midiNote;
    }

  private:
    PolyPitchDetector polyPitchDetector;

    VoiceManager voiceManager;

    std::vector<float> monoScratch;

    float bowPressure = 0.5f;
    float bowSpeed = 0.5f;
    float friction = 0.127f;

    float attackMs = 50.0f;
    float releaseMs = 200.0f;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(ElectroBowAudioProcessor)
};
#pragma once

#include <juce_audio_utils/juce_audio_utils.h>

#include <electrobow_dsp.h>

#include <array>
#include <memory>

#include <atomic>

class ElectroBowAudioProcessor : public juce::AudioProcessor {
  public:
    ElectroBowAudioProcessor();
    ~ElectroBowAudioProcessor() override = default;

    void prepareToPlay(double sampleRate, int samplesPerBlock) override;

    void releaseResources() override;

    bool isBusesLayoutSupported(const BusesLayout& layouts) const override;

    void processBlock(juce::AudioBuffer<float>&, juce::MidiBuffer&) override;

    void processBlockBypassed(juce::AudioBuffer<float>&, juce::MidiBuffer&) override;
    void reset() override;

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

    // Legacy pitch/voice telemetry stays idle; the signal path processes original audio.
    float getPitchFrequencyHz() const noexcept;

    float getPitchConfidence() const noexcept;

    int getPitchMidiNote() const noexcept;

    int getActiveVoiceCount() const noexcept;

    float getVoiceFrequencyHz(int index) const noexcept;

    float getVoiceStrength(int index) const noexcept;

    // The WebView and the audio callback read the host parameters directly.
    juce::var getUiState() const;

    void setUiParameter(const juce::String& parameterId, float value) noexcept;

    void setUiGesture(const juce::String& parameterId, bool active);
    void endUiGestures();

  private:
    // EBOW v5 order. VST3 automation uses the stable string-based parameter IDs.
    enum Control { attack, character, noise, saturation, movement, out, numControls };
    static constexpr int maxChannels = 2;
    std::array<juce::AudioParameterFloat*, numControls> controls{};
    std::array<bool, numControls> uiGestures{};
    std::unique_ptr<EbInstrument, decltype(&eb_instrument_destroy)> instrument{
        nullptr, eb_instrument_destroy};
    std::atomic<float> inputLevel{0.0f};
    juce::AudioBuffer<float> wetBuffer;
    juce::SmoothedValue<float> bypassMix;
    bool firstBlock = true;
    void render(juce::AudioBuffer<float>& buffer, bool bypassed);

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(ElectroBowAudioProcessor)
};

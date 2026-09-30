#pragma once

#include "PluginProcessor.h"

class ElectroBowAudioProcessorEditor : public juce::AudioProcessorEditor, private juce::Timer {
  public:
    explicit ElectroBowAudioProcessorEditor(ElectroBowAudioProcessor&);
    ~ElectroBowAudioProcessorEditor() override = default;

    void paint(juce::Graphics&) override;
    void resized() override;

  private:
    ElectroBowAudioProcessor& processor;

    juce::Label titleLabel;
    juce::Label noteLabel;
    juce::Label freqLabel;

    // Polls the processor's lock-free pitch atomics on the message thread.
    // 30 Hz is plenty for a stable readout and keeps the UI thread light.
    void timerCallback() override;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(ElectroBowAudioProcessorEditor)
};
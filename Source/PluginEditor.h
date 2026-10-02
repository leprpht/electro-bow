#pragma once

#include "PluginProcessor.h"

#include <juce_gui_extra/juce_gui_extra.h>

#include <memory>
#include <optional>

class ElectroBowAudioProcessorEditor : public juce::AudioProcessorEditor, private juce::Timer {
  public:
    explicit ElectroBowAudioProcessorEditor(ElectroBowAudioProcessor&);
    ~ElectroBowAudioProcessorEditor() override;

    void paint(juce::Graphics&) override;
    void resized() override;

  private:
    ElectroBowAudioProcessor& processorRef;
    std::unique_ptr<juce::WebBrowserComponent> webComponent;

    std::optional<juce::WebBrowserComponent::Resource> getResource(const juce::String& url) const;

    void timerCallback() override;

    JUCE_DECLARE_NON_COPYABLE_WITH_LEAK_DETECTOR(ElectroBowAudioProcessorEditor)
};

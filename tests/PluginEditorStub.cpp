#include "../Source/PluginEditor.h"

ElectroBowAudioProcessorEditor::ElectroBowAudioProcessorEditor(ElectroBowAudioProcessor& processor)
    : AudioProcessorEditor(&processor), processorRef(processor) {
    setSize(1, 1);
}

ElectroBowAudioProcessorEditor::~ElectroBowAudioProcessorEditor() = default;

void ElectroBowAudioProcessorEditor::paint(juce::Graphics& graphics) {
    graphics.fillAll(juce::Colours::black);
}

void ElectroBowAudioProcessorEditor::resized() {}

void ElectroBowAudioProcessorEditor::timerCallback() {}

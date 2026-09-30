#include "PluginEditor.h"
#include <cstdio>

namespace {
constexpr const char* noteNames[] = {"C",  "C#", "D",  "D#", "E",  "F",
                                     "F#", "G",  "G#", "A",  "A#", "B"};
}

ElectroBowAudioProcessorEditor::ElectroBowAudioProcessorEditor(ElectroBowAudioProcessor& p)
    : AudioProcessorEditor(&p), processor(p) {
    titleLabel.setText("ElectroBow", juce::dontSendNotification);
    titleLabel.setFont(juce::Font(juce::FontOptions(32.0f)));
    titleLabel.setColour(juce::Label::textColourId, juce::Colours::white);
    titleLabel.setJustificationType(juce::Justification::centred);
    addAndMakeVisible(titleLabel);

    noteLabel.setText("Note: ---", juce::dontSendNotification);
    noteLabel.setFont(juce::Font(juce::FontOptions(22.0f)));
    noteLabel.setColour(juce::Label::textColourId, juce::Colours::lightgrey);
    noteLabel.setJustificationType(juce::Justification::centred);
    addAndMakeVisible(noteLabel);

    freqLabel.setText("Frequency: --- Hz", juce::dontSendNotification);
    freqLabel.setFont(juce::Font(juce::FontOptions(22.0f)));
    freqLabel.setColour(juce::Label::textColourId, juce::Colours::lightgrey);
    freqLabel.setJustificationType(juce::Justification::centred);
    addAndMakeVisible(freqLabel);

    setSize(400, 300);

    // 30 Hz refresh: frequent enough for a steady readout, cheap for the UI thread.
    startTimerHz(30);
}

void ElectroBowAudioProcessorEditor::timerCallback() {
    const float freq = processor.getPitchFrequencyHz();
    const int note = processor.getPitchMidiNote();

    if (freq > 0.0f && note >= 0 && note <= 127) {
        const int pitchClass = note % 12;
        const int octave = (note / 12) - 1;

        const juce::String noteText =
            "Note: " + juce::String(noteNames[pitchClass]) + juce::String(octave);

        const juce::String freqText = "Frequency: " + juce::String(freq, 1) + " Hz";

        noteLabel.setText(noteText, juce::dontSendNotification);
        freqLabel.setText(freqText, juce::dontSendNotification);
    } else {
        noteLabel.setText("Note: ---", juce::dontSendNotification);
        freqLabel.setText("Frequency: --- Hz", juce::dontSendNotification);
    }
} // a

void ElectroBowAudioProcessorEditor::paint(juce::Graphics& g) {
    g.fillAll(juce::Colour(0xff1c1c1c));

    g.setColour(juce::Colours::white);
    g.setFont(juce::Font(juce::FontOptions(32.0f)));
    g.drawText("ElectroBow", getLocalBounds(), juce::Justification::centred);
}

void ElectroBowAudioProcessorEditor::resized() {
    auto area = getLocalBounds().reduced(10);

    titleLabel.setBounds(area.removeFromTop(50));
    area.removeFromTop(30);
    noteLabel.setBounds(area.removeFromTop(36));
    freqLabel.setBounds(area.removeFromTop(36));
}
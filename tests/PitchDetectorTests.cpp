#include "../Source/PolyPitchDetector.h"
#include "../Source/PolyphonicAnalyzer.h"

#include <cmath>
#include <iostream>
#include <vector>

namespace {
constexpr double sampleRate = 44100.0;
constexpr float pi = 3.14159265358979323846f;

float midiToFrequency(int midi) {
    return 440.0f * std::pow(2.0f, static_cast<float>(midi - 69) / 12.0f);
}

void addNote(std::vector<float>& buffer, int midiNote, float amplitude) {
    const float frequency = midiToFrequency(midiNote);

    for (size_t i = 0; i < buffer.size(); ++i) {
        const float t = static_cast<float>(i) / static_cast<float>(sampleRate);

        buffer[i] += std::sin(2.0f * pi * frequency * t) * amplitude;
    }
}

void printDetected(const char* name, PolyPitchDetector& detector) {
    std::cout << name << ": ";

    if (detector.getNumNotes() == 0) {
        std::cout << "(none)\n";
        return;
    }

    for (int i = 0; i < detector.getNumNotes(); ++i) {
        const auto note = detector.getNote(i);

        std::cout << note.midiNote << "(" << note.strength << ") ";
    }

    std::cout << '\n';
}

void testChord(const char* name, const std::vector<int>& notes) {
    constexpr int numSamples = 8192;

    std::vector<float> buffer(numSamples, 0.0f);

    for (const int midiNote : notes)
        addNote(buffer, midiNote, 0.2f);

    PolyPitchDetector detector;
    detector.prepare(sampleRate);

    detector.push(buffer.data(), static_cast<int>(buffer.size()));

    printDetected(name, detector);
}
} // namespace

int main() {
    std::cout << "ElectroBow PolyPitchDetector tests\n"
              << "-----------------------------------\n";

    // D3
    testChord("D3", {50});

    // A3
    testChord("A3", {57});

    // D3 + A3
    testChord("D3 + A3", {50, 57});

    // D3 + A3 + D4 + F4
    testChord("D3 + A3 + D4 + F4", {50, 57, 62, 65});

    // D3 + G3 + C4 + F4
    testChord("D3 + G3 + C4 + F4", {50, 55, 60, 65});

    // The analyzer must expose independent buffers and invoke the tracker
    // independently for each spectral peak.
    int trackerCalls = 0;
    PolyphonicAnalyzer analyzer([&trackerCalls](const std::vector<float>& voice, double rate) {
        ++trackerCalls;
        return PolyphonicAnalyzer::PitchEstimate{440.0f,
                                                 voice.empty() || rate <= 0.0 ? 0.0f : 1.0f};
    });
    analyzer.prepare(sampleRate);
    std::vector<float> chord(PolyphonicAnalyzer::kFFTSize * 2, 0.0f);
    addNote(chord, 50, 0.2f);
    addNote(chord, 57, 0.2f);
    analyzer.push(chord.data(), static_cast<int>(chord.size()));
    if (analyzer.getVoices().empty() ||
        trackerCalls != static_cast<int>(analyzer.getVoices().size())) {
        std::cerr << "Polyphonic analyzer did not produce independent tracked voices\n";
        return 1;
    }

    return 0;
}

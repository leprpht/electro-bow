#include "../Source/PolyPitchDetector.h"

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

    return 0;
}

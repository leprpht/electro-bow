#include "../Source/PolyPitchDetector.h"
#include "../Source/PolyphonicAnalyzer.h"

#include <algorithm>
#include <cmath>
#include <iostream>
#include <random>
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

void addGuitarLikeNote(std::vector<float>& buffer, float frequency, float amplitude,
                       float fundamentalScale = 1.0f) {
    for (size_t i = 0; i < buffer.size(); ++i) {
        const float t = static_cast<float>(i) / static_cast<float>(sampleRate);
        buffer[i] += amplitude * (fundamentalScale * 0.15f * std::sin(2.0f * pi * frequency * t) +
                                  0.70f * std::sin(4.0f * pi * frequency * t) +
                                  0.45f * std::sin(6.0f * pi * frequency * t) +
                                  0.30f * std::sin(8.0f * pi * frequency * t));
    }
}

float rms(const std::vector<float>& samples) {
    float energy = 0.0f;
    for (const float sample : samples)
        energy += sample * sample;
    return std::sqrt(energy / static_cast<float>(std::max<size_t>(1, samples.size())));
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

    // The analyzer must expose independent buffers and invoke the tracker for
    // every voice on every analysis frame.
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
    const auto& voices = analyzer.getVoices();
    if (voices.size() < 2 || trackerCalls < static_cast<int>(voices.size())) {
        std::cerr << "Polyphonic analyzer did not produce independent tracked voices\n";
        return 1;
    }

    if (voices[0].samples.empty() || voices[1].samples.empty() ||
        voices[0].samples == voices[1].samples || voices[0].id == voices[1].id) {
        std::cerr << "Polyphonic analyzer returned non-independent voice buffers\n";
        return 1;
    }

    for (const auto& voice : voices) {
        if (std::abs(voice.trackedFrequencyHz - 440.0f) > 0.01f ||
            voice.trackerConfidence <= 0.0f) {
            std::cerr << "Polyphonic analyzer did not run the pluggable tracker\n";
            return 1;
        }
    }

    const auto firstIds = std::vector<int>{voices[0].id, voices[1].id};
    analyzer.push(chord.data(), static_cast<int>(chord.size()));
    const auto& nextVoices = analyzer.getVoices();
    if (nextVoices.size() < 2 || nextVoices[0].id == nextVoices[1].id) {
        std::cerr << "Polyphonic analyzer did not maintain distinct voice slots\n";
        return 1;
    }
    for (const auto& voice : nextVoices) {
        if (std::find(firstIds.begin(), firstIds.end(), voice.id) == firstIds.end()) {
            std::cerr << "Polyphonic analyzer did not track voice identity across frames\n";
            return 1;
        }
    }

    // The tracker must receive a genuinely isolated channel and return a
    // continuous frequency, not only a MIDI-semitone estimate.
    PolyphonicAnalyzer trackedAnalyzer([](const std::vector<float>& voice, double rate) {
        PolyPitchDetector tracker;
        tracker.prepare(rate);
        tracker.push(voice.data(), static_cast<int>(voice.size()));
        return PolyphonicAnalyzer::PitchEstimate{tracker.getBestFrequencyHz(),
                                                 tracker.getBestConfidence()};
    });
    trackedAnalyzer.prepare(sampleRate);
    trackedAnalyzer.push(chord.data(), static_cast<int>(chord.size()));
    bool foundTrackedD3 = false;
    bool foundTrackedA3 = false;
    for (const auto& voice : trackedAnalyzer.getVoices()) {
        foundTrackedD3 |= std::abs(voice.trackedFrequencyHz - midiToFrequency(50)) < 2.0f;
        foundTrackedA3 |= std::abs(voice.trackedFrequencyHz - midiToFrequency(57)) < 2.0f;
    }
    if (!foundTrackedD3 || !foundTrackedA3) {
        std::cerr << "Isolated pitch tracking did not recover both continuous frequencies\n";
        return 1;
    }

    if (rms(trackedAnalyzer.getVoices()[0].samples) <= 0.0f ||
        rms(trackedAnalyzer.getVoices()[1].samples) <= 0.0f ||
        trackedAnalyzer.getVoices()[0].samples == trackedAnalyzer.getVoices()[1].samples) {
        std::cerr << "Separated voice buffers did not contain independent signal energy\n";
        return 1;
    }

    // Guitar-like harmonic content with a weak fundamental should still
    // produce the two underlying voices rather than their dominant partials.
    std::vector<float> guitarChord(PolyphonicAnalyzer::kFFTSize * 2, 0.0f);
    addGuitarLikeNote(guitarChord, 82.41f, 1.0f, 0.15f);
    addGuitarLikeNote(guitarChord, 110.0f, 1.0f, 0.15f);
    PolyphonicAnalyzer guitarAnalyzer;
    guitarAnalyzer.prepare(sampleRate);
    guitarAnalyzer.push(guitarChord.data(), static_cast<int>(guitarChord.size()));
    bool foundLowString = false;
    bool foundAString = false;
    for (const auto& voice : guitarAnalyzer.getVoices()) {
        foundLowString |= std::abs(voice.peakFrequencyHz - 82.41f) < 3.0f;
        foundAString |= std::abs(voice.peakFrequencyHz - 110.0f) < 3.0f;
    }
    if (!foundLowString || !foundAString || guitarAnalyzer.getVoices().size() > 2) {
        std::cerr << "Guitar-like harmonic separation produced incorrect voices\n";
        return 1;
    }

    std::vector<float> weakFundamental(PolyphonicAnalyzer::kFFTSize * 2, 0.0f);
    addGuitarLikeNote(weakFundamental, 82.41f, 1.0f, 0.08f);
    PolyphonicAnalyzer weakAnalyzer;
    weakAnalyzer.prepare(sampleRate);
    weakAnalyzer.push(weakFundamental.data(), static_cast<int>(weakFundamental.size()));
    bool foundWeakFundamental = false;
    for (const auto& voice : weakAnalyzer.getVoices())
        foundWeakFundamental |= std::abs(voice.peakFrequencyHz - 82.41f) < 3.0f;
    if (!foundWeakFundamental) {
        std::cerr << "Weak fundamental was not recovered from its harmonics\n";
        return 1;
    }

    // Continuous tracking must follow a bend and reject unpitched noise.
    std::vector<float> tone(PolyPitchDetector::kFFTSize, 0.0f);
    addGuitarLikeNote(tone, 440.0f, 1.0f, 1.0f);
    PolyPitchDetector continuousTracker;
    continuousTracker.prepare(sampleRate);
    continuousTracker.push(tone.data(), static_cast<int>(tone.size()));
    if (std::abs(continuousTracker.getBestFrequencyHz() - 440.0f) > 2.0f ||
        continuousTracker.getBestConfidence() < 0.5f) {
        std::cerr << "Continuous pitch estimate failed on a stable tone\n";
        return 1;
    }

    std::fill(tone.begin(), tone.end(), 0.0f);
    addGuitarLikeNote(tone, 466.16f, 1.0f, 1.0f);
    continuousTracker.push(tone.data(), static_cast<int>(tone.size()));
    if (std::abs(continuousTracker.getBestFrequencyHz() - 466.16f) > 2.0f) {
        std::cerr << "Continuous pitch estimate failed on a bend\n";
        return 1;
    }

    std::mt19937 random(7);
    std::normal_distribution<float> noise(0.0f, 0.2f);
    for (float& sample : tone)
        sample = noise(random);
    continuousTracker.push(tone.data(), static_cast<int>(tone.size()));
    if (continuousTracker.getBestFrequencyHz() != 0.0f ||
        continuousTracker.getBestConfidence() != 0.0f) {
        std::cerr << "Noise was incorrectly reported as a stable pitch\n";
        return 1;
    }

    PolyphonicAnalyzer noiseAnalyzer;
    noiseAnalyzer.prepare(sampleRate);
    noiseAnalyzer.push(tone.data(), static_cast<int>(tone.size()));
    if (!noiseAnalyzer.getVoices().empty()) {
        std::cerr << "Spectral analyzer created voices from unpitched noise\n";
        return 1;
    }

    // With no tracker installed, the analyzer still has a useful spectral
    // fallback. It should expose the two independent fundamentals rather than
    // returning only one dominant FFT peak.
    PolyphonicAnalyzer spectralAnalyzer;
    spectralAnalyzer.prepare(sampleRate);
    spectralAnalyzer.push(chord.data(), static_cast<int>(chord.size()));
    bool foundD3 = false;
    bool foundA3 = false;
    for (const auto& voice : spectralAnalyzer.getVoices()) {
        foundD3 |= std::abs(voice.peakFrequencyHz - midiToFrequency(50)) < 10.0f;
        foundA3 |= std::abs(voice.peakFrequencyHz - midiToFrequency(57)) < 10.0f;
    }
    if (!foundD3 || !foundA3) {
        std::cerr << "Polyphonic analyzer did not separate the expected fundamentals\n";
        return 1;
    }

    return 0;
}

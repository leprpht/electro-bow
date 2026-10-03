#include "../Source/BowEnvelope.h"
#include "../Source/InputDynamics.h"
#include "../Source/PitchDetector.h"
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

    for (std::size_t i = 0; i < buffer.size(); ++i) {
        const float t = static_cast<float>(i) / static_cast<float>(sampleRate);

        buffer[i] += std::sin(2.0f * pi * frequency * t) * amplitude;
    }
}

void addGuitarLikeNote(std::vector<float>& buffer, float frequency, float amplitude,
                       float fundamentalScale = 1.0f) {
    for (std::size_t i = 0; i < buffer.size(); ++i) {
        const float t = static_cast<float>(i) / static_cast<float>(sampleRate);

        buffer[i] += amplitude * (fundamentalScale * 0.15f * std::sin(2.0f * pi * frequency * t) +
                                  0.70f * std::sin(4.0f * pi * frequency * t) +
                                  0.45f * std::sin(6.0f * pi * frequency * t) +
                                  0.30f * std::sin(8.0f * pi * frequency * t));
    }
}

float rms(const std::vector<float>& samples) {
    double energy = 0.0;

    for (const float sample : samples)
        energy += static_cast<double>(sample) * sample;

    return static_cast<float>(
        std::sqrt(energy / static_cast<double>(std::max<std::size_t>(1, samples.size()))));
}

bool approximately(float a, float b, float tolerance) {
    return std::abs(a - b) <= tolerance;
}

bool containsFrequency(const PolyphonicAnalyzer& analyzer, float targetFrequency, float tolerance) {
    for (const auto& voice : analyzer.getVoices()) {
        if (approximately(voice.trackedFrequencyHz, targetFrequency, tolerance)) {
            return true;
        }
    }

    return false;
}

bool containsPeak(const PolyphonicAnalyzer& analyzer, float targetFrequency, float tolerance) {
    for (const auto& voice : analyzer.getVoices()) {
        if (approximately(voice.peakFrequencyHz, targetFrequency, tolerance)) {
            return true;
        }
    }

    return false;
}

void printPitchResult(const char* name, const PitchDetector& detector) {
    std::cout << name << ": " << detector.getFrequencyHz() << " Hz"
              << "  confidence=" << detector.getConfidence() << "  midi=" << detector.getMidiNote()
              << '\n';
}

void testSinglePitch(float frequency, const char* name) {
    constexpr int numSamples = PitchDetector::kWindowSize;

    std::vector<float> buffer(static_cast<std::size_t>(numSamples), 0.0f);

    addGuitarLikeNote(buffer, frequency, 1.0f, 1.0f);

    PitchDetector detector;
    detector.prepare(sampleRate);

    detector.push(buffer.data(), static_cast<int>(buffer.size()));

    printPitchResult(name, detector);

    if (!approximately(detector.getFrequencyHz(), frequency, 2.0f)) {
        std::cerr << "PitchDetector failed to detect " << frequency << " Hz\n";

        std::exit(1);
    }

    if (detector.getConfidence() <= 0.0f) {
        std::cerr << "PitchDetector returned zero confidence for " << frequency << " Hz\n";

        std::exit(1);
    }
}

} // namespace

int main() {
    std::cout << "ElectroBow PitchDetector / PolyphonicAnalyzer tests\n"
              << "----------------------------------------------------\n";

    // ------------------------------------------------------------
    // 1. Basic continuous pitch detection
    // ------------------------------------------------------------

    testSinglePitch(midiToFrequency(50), "D3");

    testSinglePitch(midiToFrequency(57), "A3");

    // ------------------------------------------------------------
    // 2. Continuous pitch must not be MIDI-quantized
    // ------------------------------------------------------------

    {
        constexpr float bendFrequency = 466.16f;

        std::vector<float> tone(PitchDetector::kWindowSize, 0.0f);

        addGuitarLikeNote(tone, bendFrequency, 1.0f, 1.0f);

        PitchDetector detector;
        detector.prepare(sampleRate);

        detector.push(tone.data(), static_cast<int>(tone.size()));

        if (!approximately(detector.getFrequencyHz(), bendFrequency, 2.0f)) {
            std::cerr << "Continuous pitch tracking failed on 466.16 Hz\n";

            return 1;
        }

        /*
            466.16 Hz is approximately A#4/Bb4.

            The important part is that the actual frequency remains
            466.16 Hz instead of being converted to a MIDI-grid value.
        */

        if (detector.getFrequencyHz() <= 0.0f) {
            std::cerr << "Continuous pitch detector returned no frequency\n";

            return 1;
        }

        std::cout << "Continuous bend: " << detector.getFrequencyHz() << " Hz\n";
    }

    // ------------------------------------------------------------
    // 3. Detector must update repeatedly on 1024-sample hops
    // ------------------------------------------------------------

    {
        std::vector<float> first(PitchDetector::kWindowSize, 0.0f);

        std::vector<float> second(PitchDetector::kHopSize, 0.0f);

        addGuitarLikeNote(first, 220.0f, 1.0f, 1.0f);

        addGuitarLikeNote(second, 246.94f, 1.0f, 1.0f);

        PitchDetector detector;
        detector.prepare(sampleRate);

        detector.push(first.data(), static_cast<int>(first.size()));

        const float firstFrequency = detector.getFrequencyHz();

        if (!approximately(firstFrequency, 220.0f, 2.0f)) {
            std::cerr << "Initial pitch detection failed\n";

            return 1;
        }

        /*
            Push enough new material to force another analysis.
        */

        detector.push(second.data(), static_cast<int>(second.size()));

        const float secondFrequency = detector.getFrequencyHz();

        /*
            Because the analysis window still contains old material,
            the exact transition will not necessarily be 246.94 Hz
            immediately. We only require that the detector remains
            valid and does not produce an invalid frequency.
        */

        if (!std::isfinite(secondFrequency) || secondFrequency <= 0.0f) {
            std::cerr << "PitchDetector produced an invalid frequency after hop\n";

            return 1;
        }
    }

    // ------------------------------------------------------------
    // 4. Silence must produce no pitch
    // ------------------------------------------------------------

    {
        std::vector<float> silence(PitchDetector::kWindowSize, 0.0f);

        PitchDetector detector;
        detector.prepare(sampleRate);

        detector.push(silence.data(), static_cast<int>(silence.size()));

        if (detector.getFrequencyHz() != 0.0f || detector.getConfidence() != 0.0f) {
            std::cerr << "Silence was incorrectly detected as pitch\n";

            return 1;
        }
    }

    // ------------------------------------------------------------
    // 5. Noise must not produce a stable pitch
    // ------------------------------------------------------------

    {
        std::vector<float> noiseBuffer(PitchDetector::kWindowSize, 0.0f);

        std::mt19937 random(7);
        std::normal_distribution<float> noise(0.0f, 0.2f);

        for (float& sample : noiseBuffer)
            sample = noise(random);

        PitchDetector detector;
        detector.prepare(sampleRate);

        detector.push(noiseBuffer.data(), static_cast<int>(noiseBuffer.size()));

        if (detector.getFrequencyHz() != 0.0f || detector.getConfidence() != 0.0f) {
            std::cerr << "Noise was incorrectly reported as stable pitch\n";

            return 1;
        }
    }

    // ------------------------------------------------------------
    // 6. Analyzer refreshes must not truncate a configured bow attack
    // ------------------------------------------------------------

    {
        BowEnvelope envelope;
        envelope.prepare(sampleRate);
        envelope.setAttackMs(50.0f);
        envelope.trigger(1.0f);

        for (int i = 0; i < 512; ++i)
            envelope.process();

        envelope.sustain();

        const float levelAfterRefresh = envelope.process();

        if (levelAfterRefresh >= 0.5f) {
            std::cerr << "Analyzer refresh truncated the bow attack\n";
            return 1;
        }
    }

    // ------------------------------------------------------------
    // 7. Musical input dynamics are independent from note triggering
    // ------------------------------------------------------------

    {
        InputDynamics dynamics;
        dynamics.prepare(sampleRate);

        for (int i = 0; i < 2048; ++i)
            dynamics.processSample(0.05f);

        const float softLevel = dynamics.getLevel();

        for (int i = 0; i < 4096; ++i)
            dynamics.processSample(1.0f);

        const float hardLevel = dynamics.getLevel();

        for (int i = 0; i < 4096; ++i)
            dynamics.processSample(0.05f);

        const float returnedLevel = dynamics.getLevel();

        if (!(hardLevel > softLevel * 2.0f) || !(returnedLevel < hardLevel)) {
            std::cerr << "Input dynamics follower did not track soft/hard/soft input\n";
            return 1;
        }
    }

    // ------------------------------------------------------------
    // 7. Polyphonic analyzer must produce independent voices
    // ------------------------------------------------------------

    {
        constexpr int numSamples = PolyphonicAnalyzer::kFFTSize * 2;

        std::vector<float> chord(static_cast<std::size_t>(numSamples), 0.0f);

        addNote(chord, 50, 0.2f); // D3
        addNote(chord, 57, 0.2f); // A3

        int trackerCalls = 0;

        PolyphonicAnalyzer analyzer([&trackerCalls](const std::vector<float>& voice, double rate) {
            ++trackerCalls;

            PitchDetector tracker;
            tracker.prepare(rate);

            if (!voice.empty()) {
                tracker.push(voice.data(), static_cast<int>(voice.size()));
            }

            return PolyphonicAnalyzer::PitchEstimate{tracker.getFrequencyHz(),
                                                     tracker.getConfidence()};
        });

        analyzer.prepare(sampleRate);

        analyzer.push(chord.data(), static_cast<int>(chord.size()));

        const auto& voices = analyzer.getVoices();

        if (voices.size() < 2) {
            std::cerr << "Polyphonic analyzer did not produce two voices\n";

            return 1;
        }

        if (trackerCalls < static_cast<int>(voices.size())) {
            std::cerr << "Pitch tracker was not called for every voice\n";

            return 1;
        }

        if (voices[0].samples.empty() || voices[1].samples.empty()) {
            std::cerr << "Analyzer returned empty voice buffers\n";

            return 1;
        }

        if (voices[0].samples == voices[1].samples) {
            std::cerr << "Analyzer returned identical voice buffers\n";

            return 1;
        }

        if (voices[0].id == voices[1].id) {
            std::cerr << "Analyzer assigned the same ID to two voices\n";

            return 1;
        }

        if (!containsFrequency(analyzer, midiToFrequency(50), 2.0f)) {
            std::cerr << "D3 was not recovered by isolated pitch tracking\n";

            return 1;
        }

        if (!containsFrequency(analyzer, midiToFrequency(57), 2.0f)) {
            std::cerr << "A3 was not recovered by isolated pitch tracking\n";

            return 1;
        }
    }

    // ------------------------------------------------------------
    // 7. Voice identity must persist between frames
    // ------------------------------------------------------------

    {
        constexpr int numSamples = PolyphonicAnalyzer::kFFTSize * 2;

        std::vector<float> chord(static_cast<std::size_t>(numSamples), 0.0f);

        addNote(chord, 50, 0.2f); // D3
        addNote(chord, 57, 0.2f); // A3

        PolyphonicAnalyzer analyzer;

        analyzer.prepare(sampleRate);

        analyzer.push(chord.data(), static_cast<int>(chord.size()));

        const auto firstVoices = analyzer.getVoices();

        if (firstVoices.size() < 2) {
            std::cerr << "Initial identity test did not produce two voices\n";

            return 1;
        }

        std::vector<int> firstIds;

        for (const auto& voice : firstVoices)
            firstIds.push_back(voice.id);

        analyzer.push(chord.data(), static_cast<int>(chord.size()));

        const auto& nextVoices = analyzer.getVoices();

        if (nextVoices.size() < 2) {
            std::cerr << "Second identity frame lost voices\n";

            return 1;
        }

        for (const auto& voice : nextVoices) {
            if (std::find(firstIds.begin(), firstIds.end(), voice.id) == firstIds.end()) {
                std::cerr << "Voice ID was not preserved across frames\n";

                return 1;
            }
        }
    }

    // ------------------------------------------------------------
    // 8. Guitar-like harmonic content
    // ------------------------------------------------------------

    {
        constexpr int numSamples = PolyphonicAnalyzer::kFFTSize * 2;

        std::vector<float> guitarChord(static_cast<std::size_t>(numSamples), 0.0f);

        addGuitarLikeNote(guitarChord, 82.41f, 1.0f, 0.15f);

        addGuitarLikeNote(guitarChord, 110.0f, 1.0f, 0.15f);

        PolyphonicAnalyzer analyzer;

        analyzer.prepare(sampleRate);

        analyzer.push(guitarChord.data(), static_cast<int>(guitarChord.size()));

        if (!containsPeak(analyzer, 82.41f, 3.0f)) {
            std::cerr << "Low guitar string fundamental was not recovered\n";

            return 1;
        }

        if (!containsPeak(analyzer, 110.0f, 3.0f)) {
            std::cerr << "A-string fundamental was not recovered\n";

            return 1;
        }

        if (analyzer.getVoices().size() > 2) {
            std::cerr << "Guitar harmonic separation produced too many voices\n";

            return 1;
        }
    }

    // ------------------------------------------------------------
    // 9. Weak fundamental recovery
    // ------------------------------------------------------------

    {
        constexpr int numSamples = PolyphonicAnalyzer::kFFTSize * 2;

        std::vector<float> weakFundamental(static_cast<std::size_t>(numSamples), 0.0f);

        addGuitarLikeNote(weakFundamental, 82.41f, 1.0f, 0.08f);

        PolyphonicAnalyzer analyzer;

        analyzer.prepare(sampleRate);

        analyzer.push(weakFundamental.data(), static_cast<int>(weakFundamental.size()));

        if (!containsPeak(analyzer, 82.41f, 3.0f)) {
            std::cerr << "Weak fundamental was not recovered\n";

            return 1;
        }
    }

    // ------------------------------------------------------------
    // 10. Spectral fallback without a pitch tracker
    // ------------------------------------------------------------

    {
        constexpr int numSamples = PolyphonicAnalyzer::kFFTSize * 2;

        std::vector<float> chord(static_cast<std::size_t>(numSamples), 0.0f);

        addNote(chord, 50, 0.2f); // D3
        addNote(chord, 57, 0.2f); // A3

        PolyphonicAnalyzer analyzer;

        analyzer.prepare(sampleRate);

        analyzer.push(chord.data(), static_cast<int>(chord.size()));

        if (!containsPeak(analyzer, midiToFrequency(50), 10.0f)) {
            std::cerr << "Spectral fallback did not recover D3\n";

            return 1;
        }

        if (!containsPeak(analyzer, midiToFrequency(57), 10.0f)) {
            std::cerr << "Spectral fallback did not recover A3\n";

            return 1;
        }
    }

    // ------------------------------------------------------------
    // 11. Four-note chord
    // ------------------------------------------------------------

    {
        constexpr int numSamples = PolyphonicAnalyzer::kFFTSize * 2;

        std::vector<float> chord(static_cast<std::size_t>(numSamples), 0.0f);

        addNote(chord, 50, 0.15f); // D3
        addNote(chord, 55, 0.15f); // G3
        addNote(chord, 60, 0.15f); // C4
        addNote(chord, 65, 0.15f); // F4

        PolyphonicAnalyzer analyzer;

        analyzer.prepare(sampleRate);

        analyzer.push(chord.data(), static_cast<int>(chord.size()));

        if (analyzer.getVoices().size() < 4) {
            std::cerr << "Four-note chord did not produce four voices\n";

            return 1;
        }

        if (!containsPeak(analyzer, midiToFrequency(50), 10.0f) ||
            !containsPeak(analyzer, midiToFrequency(55), 10.0f) ||
            !containsPeak(analyzer, midiToFrequency(60), 10.0f) ||
            !containsPeak(analyzer, midiToFrequency(65), 10.0f)) {
            std::cerr << "Four-note chord did not recover expected fundamentals\n";

            return 1;
        }
    }

    std::cout << "\nAll PitchDetector / PolyphonicAnalyzer tests passed.\n";

    return 0;
}

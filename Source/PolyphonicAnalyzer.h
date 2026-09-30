#pragma once

#include <algorithm>
#include <cmath>
#include <complex>
#include <functional>
#include <utility>
#include <vector>

// Spectral separation is deliberately independent of any pitch-tracking
// library. A tracker only receives one isolated voice at a time.
class PolyphonicAnalyzer {
  public:
    static constexpr int kFFTSize = 4096;
    static constexpr int kHopSize = 1024;
    static constexpr int kMaxVoices = 8;

    struct Voice {
        int id = 0;
        float peakFrequencyHz = 0.0f;
        float strength = 0.0f;
        std::vector<float> samples;
        float trackedFrequencyHz = 0.0f;
        float trackerConfidence = 0.0f;
    };

    struct PitchEstimate {
        float frequencyHz = 0.0f;
        float confidence = 0.0f;
    };

    using PitchTracker = std::function<PitchEstimate(const std::vector<float>&, double)>;

    explicit PolyphonicAnalyzer(PitchTracker tracker = {}) : pitchTracker(std::move(tracker)) {
        reset();
    }

    void prepare(double rate) {
        sampleRate = std::max(1.0, rate);
        reset();
    }

    void reset() {
        input.assign(kFFTSize, 0.0f);
        spectrum.assign(kFFTSize, {});
        voices.clear();
        writePosition = 0;
        samplesSinceAnalysis = 0;
    }

    void setPitchTracker(PitchTracker tracker) {
        pitchTracker = std::move(tracker);
    }

    void push(const float* samples, int count) {
        if (samples == nullptr || count <= 0)
            return;
        for (int i = 0; i < count; ++i) {
            input[static_cast<size_t>(writePosition)] = samples[i];
            writePosition = (writePosition + 1) % kFFTSize;
            if (++samplesSinceAnalysis >= kHopSize) {
                samplesSinceAnalysis = 0;
                analyse();
            }
        }
    }

    const std::vector<Voice>& getVoices() const noexcept {
        return voices;
    }

  private:
    static constexpr float pi = 3.14159265358979323846f;

    void fft(bool inverse) {
        for (int i = 1, j = 0; i < kFFTSize; ++i) {
            int bit = kFFTSize >> 1;
            for (; j & bit; bit >>= 1)
                j ^= bit;
            j ^= bit;
            if (i < j)
                std::swap(spectrum[static_cast<size_t>(i)], spectrum[static_cast<size_t>(j)]);
        }
        for (int length = 2; length <= kFFTSize; length <<= 1) {
            const float sign = inverse ? 1.0f : -1.0f;
            const auto step = std::polar(1.0f, sign * 2.0f * pi / length);
            for (int start = 0; start < kFFTSize; start += length) {
                std::complex<float> w(1.0f, 0.0f);
                for (int j = 0; j < length / 2; ++j) {
                    const auto even = spectrum[static_cast<size_t>(start + j)];
                    const auto odd = spectrum[static_cast<size_t>(start + j + length / 2)] * w;
                    spectrum[static_cast<size_t>(start + j)] = even + odd;
                    spectrum[static_cast<size_t>(start + j + length / 2)] = even - odd;
                    w *= step;
                }
            }
        }
        if (inverse)
            for (auto& value : spectrum)
                value /= static_cast<float>(kFFTSize);
    }

    void analyse() {
        for (int i = 0; i < kFFTSize; ++i) {
            const int index = (writePosition + i) % kFFTSize;
            const float window = 0.5f - 0.5f * std::cos(2.0f * pi * i / (kFFTSize - 1));
            spectrum[static_cast<size_t>(i)] = {input[static_cast<size_t>(index)] * window, 0.0f};
        }
        fft(false);

        float maximum = 0.0f;
        for (int bin = 2; bin < kFFTSize / 2 - 2; ++bin)
            maximum = std::max(maximum, std::abs(spectrum[static_cast<size_t>(bin)]));

        std::vector<int> peaks;
        for (int bin = 2; bin < kFFTSize / 2 - 2 && static_cast<int>(peaks.size()) < kMaxVoices;
             ++bin) {
            const float magnitude = std::abs(spectrum[static_cast<size_t>(bin)]);
            if (magnitude < maximum * 0.12f ||
                magnitude < std::abs(spectrum[static_cast<size_t>(bin - 1)]) ||
                magnitude < std::abs(spectrum[static_cast<size_t>(bin + 1)]))
                continue;
            peaks.push_back(bin);
        }

        voices.clear();
        for (int id = 0; id < static_cast<int>(peaks.size()); ++id) {
            const int peak = peaks[static_cast<size_t>(id)];
            const float magnitude = std::abs(spectrum[static_cast<size_t>(peak)]);
            const int width = 2;
            for (int bin = 0; bin < kFFTSize; ++bin)
                if (std::abs(bin - peak) > width && std::abs(bin - (kFFTSize - peak)) > width)
                    spectrum[static_cast<size_t>(bin)] = {};
            fft(true);

            Voice voice;
            voice.id = id;
            voice.peakFrequencyHz = peak * static_cast<float>(sampleRate) / kFFTSize;
            voice.strength = maximum > 0.0f ? magnitude / maximum : 0.0f;
            voice.samples.resize(kFFTSize);
            for (int i = 0; i < kFFTSize; ++i)
                voice.samples[static_cast<size_t>(i)] = spectrum[static_cast<size_t>(i)].real();
            if (pitchTracker) {
                const auto estimate = pitchTracker(voice.samples, sampleRate);
                voice.trackedFrequencyHz = estimate.frequencyHz;
                voice.trackerConfidence = estimate.confidence;
            }
            voices.push_back(std::move(voice));

            // Rebuild the source spectrum before isolating the next peak.
            for (int i = 0; i < kFFTSize; ++i) {
                const int index = (writePosition + i) % kFFTSize;
                const float window = 0.5f - 0.5f * std::cos(2.0f * pi * i / (kFFTSize - 1));
                spectrum[static_cast<size_t>(i)] = {input[static_cast<size_t>(index)] * window,
                                                    0.0f};
            }
            fft(false);
        }
    }

    double sampleRate = 44100.0;
    std::vector<float> input;
    std::vector<std::complex<float>> spectrum;
    std::vector<Voice> voices;
    PitchTracker pitchTracker;
    int writePosition = 0;
    int samplesSinceAnalysis = 0;
};

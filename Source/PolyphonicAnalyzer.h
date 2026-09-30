#pragma once

#include <algorithm>
#include <cmath>
#include <complex>
#include <functional>
#include <utility>
#include <vector>

// Spectral separation is independent of the pitch tracker. A tracker receives
// one stable voice slot at a time and can be replaced without changing this
// analyser.
class PolyphonicAnalyzer {
  public:
    static constexpr int kFFTSize = 4096;
    static constexpr int kHopSize = 1024;
    static constexpr int kMaxVoices = 8;

    struct PitchEstimate {
        float frequencyHz = 0.0f;
        float confidence = 0.0f;
    };
    using PitchTracker = std::function<PitchEstimate(const std::vector<float>&, double)>;

    struct Voice {
        int id = -1;
        float peakFrequencyHz = 0.0f;
        float strength = 0.0f;
        std::vector<float> samples;
        float trackedFrequencyHz = 0.0f;
        float trackerConfidence = 0.0f;
        bool active = false;
    };

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
        writePosition = samplesSinceAnalysis = samplesReceived = 0;
        nextVoiceId = 0;
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
            ++samplesReceived;
            if (++samplesSinceAnalysis >= kHopSize) {
                samplesSinceAnalysis = 0;
                if (samplesReceived >= kFFTSize)
                    analyse();
            }
        }
    }

    const std::vector<Voice>& getVoices() const noexcept {
        return voices;
    }

  private:
    static constexpr float pi = 3.14159265358979323846f;
    static constexpr float minimumFrequencyHz = 30.0f;
    static constexpr float maximumFrequencyHz = 2000.0f;
    static constexpr float minimumPeakRatio = 0.08f;
    static constexpr int peakSpacingBins = 4;

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

    std::vector<int> findPeaks(float peakThreshold) const {
        const int first =
            std::max(2, static_cast<int>(std::ceil(minimumFrequencyHz * kFFTSize / sampleRate)));
        const int last =
            std::min(kFFTSize / 2 - 2,
                     static_cast<int>(std::floor(maximumFrequencyHz * kFFTSize / sampleRate)));
        std::vector<int> candidates;
        for (int bin = first; bin <= last; ++bin) {
            const float magnitude = std::abs(spectrum[static_cast<size_t>(bin)]);
            if (magnitude < peakThreshold ||
                magnitude < std::abs(spectrum[static_cast<size_t>(bin - 1)]) ||
                magnitude < std::abs(spectrum[static_cast<size_t>(bin + 1)]))
                continue;
            candidates.push_back(bin);
        }
        std::sort(candidates.begin(), candidates.end(), [this](int a, int b) {
            return std::abs(spectrum[static_cast<size_t>(a)]) >
                   std::abs(spectrum[static_cast<size_t>(b)]);
        });
        std::vector<int> selected;
        for (const int candidate : candidates) {
            bool close = false;
            for (const int other : selected)
                close |= std::abs(candidate - other) < peakSpacingBins;
            if (!close)
                selected.push_back(candidate);
            if (static_cast<int>(selected.size()) == kMaxVoices)
                break;
        }
        return selected;
    }

    std::vector<float> currentFrame() const {
        std::vector<float> frame(static_cast<size_t>(kFFTSize));
        for (int i = 0; i < kFFTSize; ++i) {
            const int index = (writePosition + i) % kFFTSize;
            const float window = 0.5f - 0.5f * std::cos(2.0f * pi * i / (kFFTSize - 1));
            frame[static_cast<size_t>(i)] = input[static_cast<size_t>(index)] * window;
        }
        return frame;
    }

    int matchPreviousVoice(float frequency, const std::vector<bool>& used) const {
        int best = -1;
        float bestCents = 100000.0f;
        for (int i = 0; i < static_cast<int>(voices.size()); ++i) {
            if (used[static_cast<size_t>(i)] || !voices[static_cast<size_t>(i)].active)
                continue;
            const float old = voices[static_cast<size_t>(i)].peakFrequencyHz;
            if (old <= 0.0f)
                continue;
            const float cents = std::abs(1200.0f * std::log2(frequency / old));
            // A 200-cent gate tolerates FFT-bin movement and short bends while
            // still preventing neighbouring chord voices from swapping slots.
            if (cents < 200.0f && cents < bestCents) {
                best = i;
                bestCents = cents;
            }
        }
        return best;
    }

    void analyse() {
        const auto frame = currentFrame();
        for (int i = 0; i < kFFTSize; ++i)
            spectrum[static_cast<size_t>(i)] = {frame[static_cast<size_t>(i)], 0.0f};
        fft(false);
        float maximum = 0.0f;
        float average = 0.0f;
        for (int bin = 2; bin < kFFTSize / 2; ++bin)
            maximum = std::max(maximum, std::abs(spectrum[static_cast<size_t>(bin)]));
        for (int bin = 2; bin < kFFTSize / 2; ++bin)
            average += std::abs(spectrum[static_cast<size_t>(bin)]);
        average /= static_cast<float>(kFFTSize / 2 - 2);
        if (maximum <= 1.0e-7f) {
            voices.clear();
            return;
        }

        const auto peaks = findPeaks(std::max(maximum * minimumPeakRatio, average * 3.0f));
        std::vector<Voice> next;
        std::vector<bool> used(voices.size(), false);
        for (const int peak : peaks) {
            const float magnitude = std::abs(spectrum[static_cast<size_t>(peak)]);
            const float frequency = peak * static_cast<float>(sampleRate) / kFFTSize;
            const int previous = matchPreviousVoice(frequency, used);
            const auto sourceSpectrum = spectrum;
            for (int bin = 0; bin < kFFTSize; ++bin) {
                const bool keep = std::abs(bin - peak) <= peakSpacingBins ||
                                  std::abs(bin - (kFFTSize - peak)) <= peakSpacingBins;
                if (!keep)
                    spectrum[static_cast<size_t>(bin)] = {};
            }
            fft(true);
            Voice voice;
            voice.id = previous >= 0 ? voices[static_cast<size_t>(previous)].id : nextVoiceId++;
            voice.peakFrequencyHz = frequency;
            voice.strength = magnitude / maximum;
            voice.samples.resize(static_cast<size_t>(kFFTSize));
            for (int i = 0; i < kFFTSize; ++i)
                voice.samples[static_cast<size_t>(i)] = spectrum[static_cast<size_t>(i)].real();
            voice.active = true;
            if (pitchTracker) {
                const auto estimate = pitchTracker(voice.samples, sampleRate);
                voice.trackedFrequencyHz = estimate.frequencyHz;
                voice.trackerConfidence = std::clamp(estimate.confidence, 0.0f, 1.0f);
            } else {
                voice.trackedFrequencyHz = frequency;
                voice.trackerConfidence = voice.strength;
            }
            if (previous >= 0)
                used[static_cast<size_t>(previous)] = true;
            next.push_back(std::move(voice));
            spectrum = sourceSpectrum;
        }
        voices = std::move(next);
    }

    double sampleRate = 44100.0;
    std::vector<float> input;
    std::vector<std::complex<float>> spectrum;
    std::vector<Voice> voices;
    PitchTracker pitchTracker;
    int writePosition = 0;
    int samplesSinceAnalysis = 0;
    int samplesReceived = 0;
    int nextVoiceId = 0;
};

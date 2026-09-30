#pragma once

#include <algorithm>
#include <cmath>
#include <complex>
#include <functional>
#include <utility>
#include <vector>

// The analyzer owns spectral separation and voice identity. Pitch detection is
// deliberately supplied by the caller, so the old detector, Q, or another
// tracker can be evaluated on the same isolated channel later.
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
    using IndexedPitchTracker =
        std::function<PitchEstimate(int, const std::vector<float>&, double)>;

    struct Voice {
        int id = -1;
        // Fundamental candidate selected from the spectral peaks. This is the
        // frequency used for matching a channel between analysis frames.
        float peakFrequencyHz = 0.0f;
        // Strongest partial which contributed to the candidate, useful for
        // diagnostics without changing the pitch-tracker contract.
        float dominantPeakFrequencyHz = 0.0f;
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
        sourceSpectrum.assign(kFFTSize, {});
        voices.clear();
        voiceHistory.clear();
        writePosition = 0;
        samplesSinceAnalysis = 0;
        samplesReceived = 0;
        nextVoiceId = 0;
    }

    void setPitchTracker(PitchTracker tracker) {
        pitchTracker = std::move(tracker);
        indexedPitchTracker = {};
    }

    void setPitchTracker(IndexedPitchTracker tracker) {
        indexedPitchTracker = std::move(tracker);
        pitchTracker = {};
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
    static constexpr float maximumPeakFrequencyHz = 8000.0f;
    static constexpr float minimumPeakRatio = 0.06f;
    static constexpr float minimumFundamentalRatio = 0.02f;
    static constexpr int peakSpacingBins = 3;
    static constexpr int harmonicSearchRadiusBins = 2;

    struct SpectralPeak {
        int bin = 0;
        float magnitude = 0.0f;
    };

    struct Candidate {
        float frequencyHz = 0.0f;
        float score = 0.0f;
        float dominantPeakFrequencyHz = 0.0f;
        float dominantPeakMagnitude = 0.0f;
        float fundamentalMagnitude = 0.0f;
    };

    struct HistoricalVoice {
        int id = -1;
        float frequencyHz = 0.0f;
        int age = 0;
    };

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

        if (inverse) {
            for (auto& value : spectrum)
                value /= static_cast<float>(kFFTSize);
        }
    }

    std::vector<float> currentFrame() const {
        std::vector<float> frame(static_cast<size_t>(kFFTSize));
        for (int i = 0; i < kFFTSize; ++i) {
            const int index = (writePosition + i) % kFFTSize;
            const float window =
                0.5f - 0.5f * std::cos(2.0f * pi * i / static_cast<float>(kFFTSize - 1));
            frame[static_cast<size_t>(i)] = input[static_cast<size_t>(index)] * window;
        }
        return frame;
    }

    int firstAnalysisBin() const {
        return std::max(2, static_cast<int>(std::ceil(minimumFrequencyHz * kFFTSize / sampleRate)));
    }

    int lastAnalysisBin() const {
        return std::min(kFFTSize / 2 - 2, static_cast<int>(std::floor(maximumPeakFrequencyHz *
                                                                      kFFTSize / sampleRate)));
    }

    float magnitudeAtBin(int bin) const {
        if (bin < 0 || bin >= kFFTSize / 2)
            return 0.0f;
        return std::abs(spectrum[static_cast<size_t>(bin)]);
    }

    float localMagnitudeAtBin(float bin) const {
        const int center = static_cast<int>(std::lround(bin));
        float maximum = 0.0f;
        for (int offset = -harmonicSearchRadiusBins; offset <= harmonicSearchRadiusBins; ++offset)
            maximum = std::max(maximum, magnitudeAtBin(center + offset));
        return maximum;
    }

    std::vector<SpectralPeak> findPeaks(float threshold) const {
        const int first = firstAnalysisBin();
        const int last = lastAnalysisBin();
        std::vector<SpectralPeak> peaks;

        for (int bin = first; bin <= last; ++bin) {
            const float magnitude = magnitudeAtBin(bin);
            if (magnitude < threshold || magnitude < magnitudeAtBin(bin - 1) ||
                magnitude < magnitudeAtBin(bin + 1))
                continue;
            peaks.push_back({bin, magnitude});
        }

        std::sort(peaks.begin(), peaks.end(), [](const SpectralPeak& a, const SpectralPeak& b) {
            return a.magnitude > b.magnitude;
        });

        // Keep enough partials to recover a weak fundamental, while avoiding
        // a noisy spectrum turning into an unbounded number of channels.
        std::vector<SpectralPeak> selected;
        selected.reserve(static_cast<size_t>(kMaxVoices * 8));
        for (const auto& peak : peaks) {
            bool tooClose = false;
            for (const auto& other : selected) {
                if (std::abs(peak.bin - other.bin) < peakSpacingBins) {
                    tooClose = true;
                    break;
                }
            }
            if (tooClose)
                continue;
            selected.push_back(peak);
            if (selected.size() >= static_cast<size_t>(kMaxVoices * 8))
                break;
        }
        return selected;
    }

    float harmonicScore(float frequencyHz) const {
        float score = 0.0f;
        static constexpr float weights[] = {1.0f,  0.70f, 0.50f, 0.36f, 0.27f,
                                            0.20f, 0.15f, 0.11f, 0.08f, 0.06f};
        for (int harmonic = 1; harmonic <= 10; ++harmonic) {
            const float harmonicFrequency = frequencyHz * static_cast<float>(harmonic);
            if (harmonicFrequency >= sampleRate * 0.48)
                break;
            const float bin = harmonicFrequency * kFFTSize / static_cast<float>(sampleRate);
            score += localMagnitudeAtBin(bin) * weights[harmonic - 1];
        }
        return score;
    }

    Candidate estimateFundamental(const SpectralPeak& sourcePeak, float maximumMagnitude) const {
        const float sourceFrequency = sourcePeak.bin * static_cast<float>(sampleRate) / kFFTSize;
        Candidate best;

        for (int harmonic = 1; harmonic <= 8; ++harmonic) {
            const float candidateFrequency = sourceFrequency / static_cast<float>(harmonic);
            if (candidateFrequency < minimumFrequencyHz || candidateFrequency > maximumFrequencyHz)
                continue;

            // A subharmonic is accepted only when there is at least a weak
            // spectral foothold at its fundamental. This prevents a chord's
            // unrelated partials from creating spurious low voices, while
            // still supporting weak guitar fundamentals.
            if (harmonic > 1) {
                const float fundamentalBin =
                    candidateFrequency * kFFTSize / static_cast<float>(sampleRate);
                if (localMagnitudeAtBin(fundamentalBin) <
                    maximumMagnitude * minimumFundamentalRatio)
                    continue;
            }

            const float score = harmonicScore(candidateFrequency);
            if (score > best.score) {
                best.frequencyHz = candidateFrequency;
                best.score = score;
                best.dominantPeakFrequencyHz = sourceFrequency;
                best.dominantPeakMagnitude = sourcePeak.magnitude;
                best.fundamentalMagnitude = localMagnitudeAtBin(candidateFrequency * kFFTSize /
                                                                static_cast<float>(sampleRate));
            }
        }
        return best;
    }

    static bool frequenciesAreClose(float a, float b) {
        return a > 0.0f && b > 0.0f && std::abs(1200.0f * std::log2(a / b)) < 80.0f;
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
            if (cents < 250.0f && cents < bestCents) {
                best = i;
                bestCents = cents;
            }
        }
        return best;
    }

    int matchHistoricalVoice(float frequency) const {
        int bestId = -1;
        float bestCents = 100000.0f;
        for (const auto& historical : voiceHistory) {
            if (historical.frequencyHz <= 0.0f)
                continue;
            const float cents = std::abs(1200.0f * std::log2(frequency / historical.frequencyHz));
            if (cents < 250.0f && cents < bestCents) {
                bestId = historical.id;
                bestCents = cents;
            }
        }
        return bestId;
    }

    void updateVoiceHistory() {
        for (auto& historical : voiceHistory)
            ++historical.age;

        for (const auto& voice : voices) {
            auto existing = std::find_if(
                voiceHistory.begin(), voiceHistory.end(),
                [&voice](const HistoricalVoice& historical) { return historical.id == voice.id; });
            if (existing == voiceHistory.end())
                voiceHistory.push_back({voice.id, voice.peakFrequencyHz, 0});
            else {
                existing->frequencyHz = voice.peakFrequencyHz;
                existing->age = 0;
            }
        }

        voiceHistory.erase(
            std::remove_if(voiceHistory.begin(), voiceHistory.end(),
                           [](const HistoricalVoice& historical) { return historical.age > 8; }),
            voiceHistory.end());
    }

    void analyse() {
        const auto frame = currentFrame();
        for (int i = 0; i < kFFTSize; ++i)
            spectrum[static_cast<size_t>(i)] = {frame[static_cast<size_t>(i)], 0.0f};
        fft(false);

        float maximumMagnitude = 0.0f;
        float averageMagnitude = 0.0f;
        const int first = firstAnalysisBin();
        const int last = lastAnalysisBin();
        for (int bin = first; bin <= last; ++bin) {
            const float magnitude = magnitudeAtBin(bin);
            maximumMagnitude = std::max(maximumMagnitude, magnitude);
            averageMagnitude += magnitude;
        }
        averageMagnitude /= static_cast<float>(std::max(1, last - first + 1));

        if (maximumMagnitude <= 1.0e-7f) {
            voices.clear();
            updateVoiceHistory();
            return;
        }

        const auto peaks =
            findPeaks(std::max(maximumMagnitude * minimumPeakRatio, averageMagnitude * 3.0f));
        std::vector<Candidate> candidates;
        candidates.reserve(peaks.size());

        for (const auto& peak : peaks) {
            const auto candidate = estimateFundamental(peak, maximumMagnitude);
            if (candidate.frequencyHz <= 0.0f || candidate.score <= 0.0f)
                continue;

            bool merged = false;
            for (auto& existing : candidates) {
                if (frequenciesAreClose(existing.frequencyHz, candidate.frequencyHz)) {
                    if (candidate.score > existing.score)
                        existing = candidate;
                    merged = true;
                    break;
                }
            }
            if (!merged)
                candidates.push_back(candidate);
        }

        // If a note has a weak fundamental, its second/third partial can
        // otherwise look like an additional octave voice. Prefer the lower
        // candidate only in that specific weak-fundamental case; strong
        // octave notes remain independently representable.
        std::vector<Candidate> filteredCandidates;
        filteredCandidates.reserve(candidates.size());
        for (const auto& candidate : candidates) {
            bool isWeakFundamentalDuplicate = false;
            for (const auto& lower : candidates) {
                if (lower.frequencyHz >= candidate.frequencyHz || lower.frequencyHz <= 0.0f)
                    continue;
                const float ratio = candidate.frequencyHz / lower.frequencyHz;
                const int roundedRatio = static_cast<int>(std::lround(ratio));
                if (roundedRatio >= 2 && roundedRatio <= 8 &&
                    std::abs(ratio - static_cast<float>(roundedRatio)) < 0.025f &&
                    lower.fundamentalMagnitude < lower.dominantPeakMagnitude * 0.35f) {
                    isWeakFundamentalDuplicate = true;
                    break;
                }
            }
            if (!isWeakFundamentalDuplicate)
                filteredCandidates.push_back(candidate);
        }
        candidates = std::move(filteredCandidates);

        std::sort(candidates.begin(), candidates.end(),
                  [](const Candidate& a, const Candidate& b) { return a.score > b.score; });
        if (candidates.size() > static_cast<size_t>(kMaxVoices))
            candidates.resize(static_cast<size_t>(kMaxVoices));

        sourceSpectrum = spectrum;
        std::vector<Voice> next;
        next.reserve(candidates.size());
        std::vector<bool> used(voices.size(), false);

        for (const auto& candidate : candidates) {
            const int previous = matchPreviousVoice(candidate.frequencyHz, used);
            int historicalId = previous >= 0 ? voices[static_cast<size_t>(previous)].id
                                             : matchHistoricalVoice(candidate.frequencyHz);
            if (historicalId >= 0 &&
                std::any_of(next.begin(), next.end(), [historicalId](const Voice& voice) {
                    return voice.id == historicalId;
                }))
                historicalId = -1;
            spectrum = sourceSpectrum;

            // Reconstruct a channel from the fundamental and its partials.
            // This is the important distinction from forwarding one FFT bin:
            // the pitch tracker sees the complete isolated voice, including
            // harmonics that can rescue a weak fundamental.
            for (int bin = 0; bin < kFFTSize / 2; ++bin) {
                bool keep = false;
                for (int harmonic = 1; harmonic <= 10 && !keep; ++harmonic) {
                    const float partial = candidate.frequencyHz * harmonic * kFFTSize /
                                          static_cast<float>(sampleRate);
                    keep =
                        std::abs(bin - static_cast<int>(std::lround(partial))) <= peakSpacingBins;
                }
                if (!keep) {
                    spectrum[static_cast<size_t>(bin)] = {};
                    if (bin > 0)
                        spectrum[static_cast<size_t>(kFFTSize - bin)] = {};
                }
            }

            fft(true);
            Voice voice;
            voice.id = historicalId >= 0 ? historicalId : nextVoiceId++;
            voice.peakFrequencyHz = candidate.frequencyHz;
            voice.dominantPeakFrequencyHz = candidate.dominantPeakFrequencyHz;
            voice.strength =
                std::clamp(candidate.score / std::max(1.0e-7f, maximumMagnitude), 0.0f, 1.0f);
            voice.samples.resize(static_cast<size_t>(kFFTSize));
            for (int i = 0; i < kFFTSize; ++i)
                voice.samples[static_cast<size_t>(i)] = spectrum[static_cast<size_t>(i)].real();
            voice.active = true;

            PitchEstimate estimate;
            if (indexedPitchTracker)
                estimate = indexedPitchTracker(voice.id, voice.samples, sampleRate);
            else if (pitchTracker)
                estimate = pitchTracker(voice.samples, sampleRate);

            voice.trackedFrequencyHz =
                estimate.frequencyHz > 0.0f ? estimate.frequencyHz : candidate.frequencyHz;
            voice.trackerConfidence = estimate.frequencyHz > 0.0f
                                          ? std::clamp(estimate.confidence, 0.0f, 1.0f)
                                          : voice.strength;

            if (previous >= 0)
                used[static_cast<size_t>(previous)] = true;
            next.push_back(std::move(voice));
        }

        voices = std::move(next);
        updateVoiceHistory();
    }

    double sampleRate = 44100.0;
    std::vector<float> input;
    std::vector<std::complex<float>> spectrum;
    std::vector<std::complex<float>> sourceSpectrum;
    std::vector<Voice> voices;
    std::vector<HistoricalVoice> voiceHistory;
    PitchTracker pitchTracker;
    IndexedPitchTracker indexedPitchTracker;
    int writePosition = 0;
    int samplesSinceAnalysis = 0;
    int samplesReceived = 0;
    int nextVoiceId = 0;
};

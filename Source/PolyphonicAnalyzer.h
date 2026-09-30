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
        frame.assign(kFFTSize, 0.0f);
        spectrum.assign(kFFTSize, {});
        sourceSpectrum.assign(kFFTSize, {});
        voices.clear();
        voiceHistory.clear();
        spectralPeaks.clear();
        selectedPeaks.clear();
        candidateScratch.clear();
        mergedCandidateScratch.clear();
        filteredCandidateScratch.clear();
        nextVoiceScratch.clear();
        usedScratch.clear();
        voices.reserve(kMaxVoices);
        voiceHistory.reserve(kMaxVoices * 2);
        spectralPeaks.reserve(kMaxVoices * 8);
        selectedPeaks.reserve(kMaxVoices * 8);
        candidateScratch.reserve(kMaxVoices * 16);
        mergedCandidateScratch.reserve(kMaxVoices * 16);
        filteredCandidateScratch.reserve(kMaxVoices * 16);
        nextVoiceScratch.reserve(kMaxVoices);
        usedScratch.reserve(kMaxVoices);
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
    static constexpr float minimumFundamentalRatio = 0.005f;
    static constexpr int peakSpacingBins = 3;
    // Peak interpolation keeps the expected partial within roughly one bin;
    // a wider search radius incorrectly treats neighbouring low-frequency
    // fundamentals as evidence for a subharmonic.
    static constexpr int harmonicSearchRadiusBins = 1;

    struct SpectralPeak {
        int bin = 0;
        float frequencyHz = 0.0f;
        float magnitude = 0.0f;
    };

    struct Candidate {
        float frequencyHz = 0.0f;
        float score = 0.0f;
        float dominantPeakFrequencyHz = 0.0f;
        float dominantPeakMagnitude = 0.0f;
        float fundamentalMagnitude = 0.0f;
        int supportedHarmonics = 0;
        int sourceHarmonic = 1;
        bool hasFundamentalPeak = false;
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

    void fillCurrentFrame() {
        for (int i = 0; i < kFFTSize; ++i) {
            const int index = (writePosition + i) % kFFTSize;
            const float window =
                0.5f - 0.5f * std::cos(2.0f * pi * i / static_cast<float>(kFFTSize - 1));
            frame[static_cast<size_t>(i)] = input[static_cast<size_t>(index)] * window;
        }
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

    bool isLocalPeakNearBin(float bin, float threshold) const {
        const int center = static_cast<int>(std::lround(bin));
        for (int offset = -1; offset <= 1; ++offset) {
            const int peakBin = center + offset;
            const float magnitude = magnitudeAtBin(peakBin);
            if (magnitude < threshold || magnitude < magnitudeAtBin(peakBin - 1) ||
                magnitude < magnitudeAtBin(peakBin + 1))
                continue;

            float peakOffset = 0.0f;
            const float left = magnitudeAtBin(peakBin - 1);
            const float right = magnitudeAtBin(peakBin + 1);
            const float curvature = left - 2.0f * magnitude + right;
            if (std::abs(curvature) > 1.0e-6f)
                peakOffset = std::clamp(0.5f * (left - right) / curvature, -0.5f, 0.5f);

            if (std::abs(static_cast<float>(peakBin) + peakOffset - bin) <= 0.25f)
                return true;
        }
        return false;
    }

    const std::vector<SpectralPeak>& findPeaks(float threshold) {
        const int first = firstAnalysisBin();
        const int last = lastAnalysisBin();
        spectralPeaks.clear();

        for (int bin = first; bin <= last; ++bin) {
            const float magnitude = magnitudeAtBin(bin);
            if (magnitude < threshold || magnitude < magnitudeAtBin(bin - 1) ||
                magnitude < magnitudeAtBin(bin + 1))
                continue;
            float offset = 0.0f;
            const float left = magnitudeAtBin(bin - 1);
            const float center = magnitude;
            const float right = magnitudeAtBin(bin + 1);
            const float curvature = left - 2.0f * center + right;
            if (std::abs(curvature) > 1.0e-6f)
                offset = std::clamp(0.5f * (left - right) / curvature, -0.5f, 0.5f);

            spectralPeaks.push_back(
                {bin,
                 (static_cast<float>(bin) + offset) * static_cast<float>(sampleRate) / kFFTSize,
                 magnitude});
        }

        std::sort(
            spectralPeaks.begin(), spectralPeaks.end(),
            [](const SpectralPeak& a, const SpectralPeak& b) { return a.magnitude > b.magnitude; });

        // Keep enough partials to recover a weak fundamental, while avoiding
        // a noisy spectrum turning into an unbounded number of channels.
        selectedPeaks.clear();
        for (const auto& peak : spectralPeaks) {
            bool tooClose = false;
            for (const auto& other : selectedPeaks) {
                if (std::abs(peak.bin - other.bin) < peakSpacingBins) {
                    tooClose = true;
                    break;
                }
            }
            if (tooClose)
                continue;
            selectedPeaks.push_back(peak);
            if (selectedPeaks.size() >= static_cast<size_t>(kMaxVoices * 8))
                break;
        }
        return selectedPeaks;
    }

    Candidate scoreCandidate(float frequencyHz, float maximumMagnitude) const {
        Candidate result;
        result.frequencyHz = frequencyHz;
        float score = 0.0f;
        int supportedHarmonics = 0;
        static constexpr float weights[] = {1.0f,  0.70f, 0.50f, 0.36f, 0.27f,
                                            0.20f, 0.15f, 0.11f, 0.08f, 0.06f};
        for (int harmonic = 1; harmonic <= 10; ++harmonic) {
            const float harmonicFrequency = frequencyHz * static_cast<float>(harmonic);
            if (harmonicFrequency >= sampleRate * 0.48)
                break;
            const float bin = harmonicFrequency * kFFTSize / static_cast<float>(sampleRate);
            const float magnitude = localMagnitudeAtBin(bin);
            score += magnitude * weights[harmonic - 1];
            if (magnitude >= maximumMagnitude * 0.035f &&
                isLocalPeakNearBin(bin, maximumMagnitude * 0.035f))
                ++supportedHarmonics;
        }
        result.score = score;
        result.supportedHarmonics = supportedHarmonics;
        result.fundamentalMagnitude =
            localMagnitudeAtBin(frequencyHz * kFFTSize / static_cast<float>(sampleRate));
        result.hasFundamentalPeak =
            isLocalPeakNearBin(frequencyHz * kFFTSize / static_cast<float>(sampleRate),
                               maximumMagnitude * minimumFundamentalRatio);
        return result;
    }

    void appendFundamentalCandidates(const SpectralPeak& sourcePeak, float maximumMagnitude,
                                     std::vector<Candidate>& candidates) const {
        const float sourceFrequency = sourcePeak.frequencyHz;

        for (int harmonic = 1; harmonic <= 8; ++harmonic) {
            const float candidateFrequency = sourceFrequency / static_cast<float>(harmonic);
            if (candidateFrequency < minimumFrequencyHz || candidateFrequency > maximumFrequencyHz)
                continue;

            const float fundamentalMagnitude =
                localMagnitudeAtBin(candidateFrequency * kFFTSize / static_cast<float>(sampleRate));
            if (harmonic > 1 && fundamentalMagnitude < maximumMagnitude * minimumFundamentalRatio)
                continue;

            const auto scored = scoreCandidate(candidateFrequency, maximumMagnitude);
            if (harmonic > 1 && scored.supportedHarmonics < 2)
                continue;
            if (harmonic > 1 && !scored.hasFundamentalPeak && scored.supportedHarmonics < 3)
                continue;

            Candidate candidate = scored;
            candidate.dominantPeakFrequencyHz = sourceFrequency;
            candidate.dominantPeakMagnitude = sourcePeak.magnitude;
            candidate.fundamentalMagnitude = fundamentalMagnitude;
            candidate.sourceHarmonic = harmonic;
            candidates.push_back(candidate);
        }
    }

    static bool frequenciesAreClose(float a, float b) {
        return a > 0.0f && b > 0.0f && std::abs(1200.0f * std::log2(a / b)) < 45.0f;
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
        fillCurrentFrame();
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

        // A tonal spectrum has a clear peak above its broadband floor. This
        // gate prevents isolated random-noise maxima from becoming voices.
        if (maximumMagnitude <= 1.0e-7f || maximumMagnitude < averageMagnitude * 4.0f) {
            voices.clear();
            updateVoiceHistory();
            return;
        }

        const auto& peaks =
            findPeaks(std::max(maximumMagnitude * minimumPeakRatio, averageMagnitude * 3.0f));
        auto& candidates = candidateScratch;
        candidates.clear();

        for (const auto& peak : peaks)
            appendFundamentalCandidates(peak, maximumMagnitude, candidates);

        auto& mergedCandidates = mergedCandidateScratch;
        mergedCandidates.clear();
        for (const auto& candidate : candidates) {
            bool merged = false;
            for (auto& existing : mergedCandidates) {
                if (frequenciesAreClose(existing.frequencyHz, candidate.frequencyHz)) {
                    if (candidate.score > existing.score)
                        existing = candidate;
                    merged = true;
                    break;
                }
            }
            if (!merged)
                mergedCandidates.push_back(candidate);
        }
        candidates.clear();

        // A partial can still produce a lower subharmonic candidate when its
        // local search window overlaps another note. Drop that candidate only
        // when a direct, stronger higher-frequency candidate explains the same
        // harmonic relationship. If no direct candidate exists, retain it so
        // genuinely weak guitar fundamentals remain recoverable.
        auto& filteredCandidates = filteredCandidateScratch;
        filteredCandidates.clear();
        for (const auto& candidate : mergedCandidates) {
            bool isWeakSubharmonic =
                candidate.sourceHarmonic > 1 && !candidate.hasFundamentalPeak &&
                candidate.fundamentalMagnitude < candidate.dominantPeakMagnitude * 0.12f;
            bool hasDirectExplanation = false;
            if (isWeakSubharmonic) {
                for (const auto& direct : mergedCandidates) {
                    if (direct.sourceHarmonic != 1 || direct.frequencyHz <= candidate.frequencyHz)
                        continue;
                    const float ratio = direct.frequencyHz / candidate.frequencyHz;
                    const int roundedRatio = static_cast<int>(std::lround(ratio));
                    if (roundedRatio >= 2 && roundedRatio <= 10 &&
                        std::abs(ratio - static_cast<float>(roundedRatio)) < 0.08f &&
                        direct.score > candidate.score * 0.35f) {
                        hasDirectExplanation = true;
                        break;
                    }
                }
            }
            if (isWeakSubharmonic && hasDirectExplanation)
                continue;

            filteredCandidates.push_back(candidate);
        }

        // Once a candidate has several harmonics, a higher candidate whose
        // entire harmonic series is an integer multiple is normally a partial
        // of the lower voice, not a new voice. Leave one-partial candidates
        // alone: two clean sine-like tones at an octave are still distinct
        // spectral peaks and remain representable.
        candidates.clear();
        for (const auto& candidate : filteredCandidates) {
            bool explainedByLowerVoice = false;
            if (candidate.supportedHarmonics > 0) {
                for (const auto& lower : filteredCandidates) {
                    if (lower.frequencyHz >= candidate.frequencyHz || lower.supportedHarmonics < 2)
                        continue;
                    const float ratio = candidate.frequencyHz / lower.frequencyHz;
                    const int roundedRatio = static_cast<int>(std::lround(ratio));
                    if (roundedRatio >= 2 && roundedRatio <= 10 &&
                        std::abs(ratio - static_cast<float>(roundedRatio)) < 0.08f &&
                        (candidate.supportedHarmonics >= 2 || lower.supportedHarmonics >= 3)) {
                        explainedByLowerVoice = true;
                        break;
                    }
                }
            }
            if (!explainedByLowerVoice)
                candidates.push_back(candidate);
        }

        std::sort(candidates.begin(), candidates.end(),
                  [](const Candidate& a, const Candidate& b) { return a.score > b.score; });
        if (candidates.size() > static_cast<size_t>(kMaxVoices))
            candidates.resize(static_cast<size_t>(kMaxVoices));

        sourceSpectrum = spectrum;
        auto& next = nextVoiceScratch;
        next.clear();
        next.reserve(candidates.size());
        auto& used = usedScratch;
        used.assign(voices.size(), false);

        for (size_t voiceIndex = 0; voiceIndex < candidates.size(); ++voiceIndex) {
            const auto& candidate = candidates[voiceIndex];
            const int previous = matchPreviousVoice(candidate.frequencyHz, used);
            int historicalId = previous >= 0 ? voices[static_cast<size_t>(previous)].id
                                             : matchHistoricalVoice(candidate.frequencyHz);
            if (historicalId >= 0 &&
                std::any_of(next.begin(), next.end(), [historicalId](const Voice& voice) {
                    return voice.id == historicalId;
                }))
                historicalId = -1;
            spectrum = sourceSpectrum;

            // Build a normalized soft spectral mask for this candidate. A
            // source bin is assigned across competing voices instead of being
            // copied wholesale into every channel. Exact harmonic collisions
            // are inherently ambiguous, but the channels remain independent
            // and their retained energy is conserved.
            for (int bin = 0; bin < kFFTSize / 2; ++bin) {
                float weight = 0.0f;
                float totalWeight = 0.0f;
                for (size_t candidateIndex = 0; candidateIndex < candidates.size();
                     ++candidateIndex) {
                    float distance = static_cast<float>(kFFTSize);
                    for (int harmonic = 1; harmonic <= 10; ++harmonic) {
                        const float partial = candidates[candidateIndex].frequencyHz * harmonic *
                                              kFFTSize / static_cast<float>(sampleRate);
                        distance = std::min(distance, std::abs(bin - partial));
                    }
                    if (distance <= static_cast<float>(peakSpacingBins)) {
                        const float candidateWeight =
                            std::max(0.001f, static_cast<float>(peakSpacingBins + 1) - distance);
                        totalWeight += candidateWeight;
                        if (candidateIndex == voiceIndex)
                            weight = candidateWeight;
                    }
                }

                if (totalWeight <= 0.0f) {
                    spectrum[static_cast<size_t>(bin)] = {};
                    if (bin > 0)
                        spectrum[static_cast<size_t>(kFFTSize - bin)] = {};
                } else {
                    const float mask = weight / totalWeight;
                    spectrum[static_cast<size_t>(bin)] *= mask;
                    if (bin > 0)
                        spectrum[static_cast<size_t>(kFFTSize - bin)] *= mask;
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

        voices.swap(next);
        updateVoiceHistory();
    }

    double sampleRate = 44100.0;
    std::vector<float> input;
    std::vector<float> frame;
    std::vector<std::complex<float>> spectrum;
    std::vector<std::complex<float>> sourceSpectrum;
    std::vector<Voice> voices;
    std::vector<HistoricalVoice> voiceHistory;
    std::vector<SpectralPeak> spectralPeaks;
    std::vector<SpectralPeak> selectedPeaks;
    std::vector<Candidate> candidateScratch;
    std::vector<Candidate> mergedCandidateScratch;
    std::vector<Candidate> filteredCandidateScratch;
    std::vector<Voice> nextVoiceScratch;
    std::vector<bool> usedScratch;
    PitchTracker pitchTracker;
    IndexedPitchTracker indexedPitchTracker;
    int writePosition = 0;
    int samplesSinceAnalysis = 0;
    int samplesReceived = 0;
    int nextVoiceId = 0;
};

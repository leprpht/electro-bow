#pragma once

#include <algorithm>
#include <array>
#include <cmath>
#include <complex>
#include <vector>

class PolyPitchDetector {
  public:
    static constexpr int kFFTOrder = 12;
    static constexpr int kFFTSize = 1 << kFFTOrder;
    static constexpr int kHopSize = 512;
    static constexpr int kMaxNotes = 8;

    struct DetectedNote {
        int midiNote = 0;
        float strength = 0.0f;
    };

    PolyPitchDetector() {
        reset();
    }

    void prepare(double newSampleRate) {
        sampleRate = std::max(1.0, newSampleRate);
        reset();
    }

    void reset() {
        inputBuffer.fill(0.0f);
        fftBuffer.fill(std::complex<float>(0.0f, 0.0f));

        writePosition = 0;
        samplesSinceAnalysis = 0;
        samplesReceived = 0;
        samplesAtLastContinuousAnalysis = 0;

        detectedNoteCount = 0;
        bestFrequencyHz = 0.0f;
        bestConfidence = 0.0f;
    }

    void push(const float* samples, int numSamples) {
        if (samples == nullptr || numSamples <= 0) {
            return;
        }

        for (int i = 0; i < numSamples; ++i) {
            inputBuffer[static_cast<size_t>(writePosition)] = samples[i];

            writePosition = (writePosition + 1) % kFFTSize;

            ++samplesReceived;
            ++samplesSinceAnalysis;

            if (samplesSinceAnalysis >= kHopSize) {
                samplesSinceAnalysis = 0;
                analyse();
            }
        }
    }

    int getNumNotes() const noexcept {
        return detectedNoteCount;
    }

    DetectedNote getNote(int index) const noexcept {
        if (index < 0 || index >= detectedNoteCount) {
            return {};
        }

        return detectedNotes[static_cast<size_t>(index)];
    }

    // Continuous estimate used when this detector is fed an isolated voice.
    // The MIDI-note list above remains available as the legacy detector
    // interface, while this estimate preserves bends between semitones.
    float getBestFrequencyHz() const noexcept {
        return bestFrequencyHz;
    }

    float getBestConfidence() const noexcept {
        return bestConfidence;
    }

  private:
    void analyse() {
        buildFFTInput();
        performFFT();

        std::array<float, kMaxNotes> candidateStrengths{};

        std::array<int, kMaxNotes> candidateNotes{};

        float strongest = 0.0f;

        for (int midi = 36; midi <= 88; ++midi) {
            const float strength = calculateHarmonicStrength(midi);

            if (strength > strongest)
                strongest = strength;
        }

        if (strongest <= 0.000001f) {
            detectedNoteCount = 0;
            bestFrequencyHz = 0.0f;
            bestConfidence = 0.0f;
            return;
        }

        const float threshold = strongest * 0.20f;

        int candidateCount = 0;

        for (int midi = 36; midi <= 88 && candidateCount < kMaxNotes; ++midi) {
            const float strength = calculateHarmonicStrength(midi);

            if (strength < threshold)
                continue;

            bool isLocalMaximum = true;

            if (midi > 36) {
                const float previous = calculateHarmonicStrength(midi - 1);

                if (strength < previous)
                    isLocalMaximum = false;
            }

            if (midi < 88) {
                const float next = calculateHarmonicStrength(midi + 1);

                if (strength < next)
                    isLocalMaximum = false;
            }

            if (!isLocalMaximum)
                continue;

            candidateNotes[static_cast<size_t>(candidateCount)] = midi;

            candidateStrengths[static_cast<size_t>(candidateCount)] = strength / strongest;

            ++candidateCount;
        }

        std::array<DetectedNote, kMaxNotes> newNotes{};

        for (int i = 0; i < candidateCount; ++i) {
            newNotes[static_cast<size_t>(i)].midiNote = candidateNotes[static_cast<size_t>(i)];
            newNotes[static_cast<size_t>(i)].strength = candidateStrengths[static_cast<size_t>(i)];
        }

        std::sort(
            newNotes.begin(), newNotes.begin() + candidateCount,
            [](const DetectedNote& a, const DetectedNote& b) { return a.strength > b.strength; });

        detectedNotes = newNotes;
        detectedNoteCount = candidateCount;

        if (samplesReceived >= kFFTSize &&
            samplesReceived - samplesAtLastContinuousAnalysis >= kFFTSize) {
            estimateContinuousPitch();
            samplesAtLastContinuousAnalysis = samplesReceived;
        }
    }

    void estimateContinuousPitch() {
        bestFrequencyHz = 0.0f;
        bestConfidence = 0.0f;

        if (samplesReceived < kFFTSize)
            return;

        constexpr float minimumConfidence = 0.30f;

        const int minimumLag = std::max(2, static_cast<int>(std::floor(sampleRate / 2000.0)));
        const int maximumLag =
            std::min(kFFTSize / 2, static_cast<int>(std::ceil(sampleRate / 30.0)));
        if (maximumLag <= minimumLag + 2)
            return;

        // The forward FFT has already been computed by analyse(). The inverse
        // transform of its power spectrum is the autocorrelation, avoiding a
        // second O(N * lag) time-domain pass for every isolated voice.
        for (auto& value : fftBuffer)
            value = {std::norm(value), 0.0f};
        performFFT(true);

        const float zeroLag = fftBuffer[0].real();
        if (zeroLag <= 1.0e-8f)
            return;

        float bestCorrelation = 0.0f;
        int bestLag = 0;
        std::array<float, kFFTSize / 2 + 1> correlations{};

        for (int lag = minimumLag; lag <= maximumLag; ++lag) {
            const float correlation = fftBuffer[static_cast<size_t>(lag)].real() / zeroLag;
            correlations[static_cast<size_t>(lag)] = correlation;
            if (correlation > bestCorrelation) {
                bestCorrelation = correlation;
                bestLag = lag;
            }
        }

        if (bestLag == 0 || bestCorrelation < minimumConfidence)
            return;

        // Prefer the first strong local maximum. This avoids selecting an
        // octave multiple simply because several periods correlate well.
        const float localThreshold = bestCorrelation * 0.97f;
        int selectedLag = bestLag;
        for (int lag = minimumLag + 1; lag < maximumLag; ++lag) {
            const float current = correlations[static_cast<size_t>(lag)];
            if (current >= localThreshold &&
                current >= correlations[static_cast<size_t>(lag - 1)] &&
                current >= correlations[static_cast<size_t>(lag + 1)]) {
                selectedLag = lag;
                break;
            }
        }

        float refinedLag = static_cast<float>(selectedLag);
        if (selectedLag > minimumLag && selectedLag < maximumLag) {
            const float left = correlations[static_cast<size_t>(selectedLag - 1)];
            const float center = correlations[static_cast<size_t>(selectedLag)];
            const float right = correlations[static_cast<size_t>(selectedLag + 1)];
            const float curvature = left - 2.0f * center + right;
            if (std::abs(curvature) > 1.0e-6f)
                refinedLag += 0.5f * (left - right) / curvature;
        }

        if (refinedLag <= 0.0f)
            return;

        bestFrequencyHz = static_cast<float>(sampleRate / refinedLag);
        bestConfidence = std::clamp(
            (bestCorrelation - minimumConfidence) / (1.0f - minimumConfidence), 0.0f, 1.0f);
    }

    void buildFFTInput() {
        constexpr float pi = 3.14159265358979323846f;

        for (int i = 0; i < kFFTSize; ++i) {
            const int index = (writePosition + i) % kFFTSize;

            const float sample = inputBuffer[static_cast<size_t>(index)];

            const float phase = static_cast<float>(i) / static_cast<float>(kFFTSize - 1);

            const float window = 0.5f * (1.0f - std::cos(2.0f * pi * phase));

            fftBuffer[static_cast<size_t>(i)] = std::complex<float>(sample * window, 0.0f);
        }
    }

    void performFFT(bool inverse = false) {
        constexpr float pi = 3.14159265358979323846f;

        // Bit reversal.
        for (int i = 1, j = 0; i < kFFTSize; ++i) {
            int bit = kFFTSize >> 1;

            for (; j & bit; bit >>= 1) {
                j ^= bit;
            }

            j ^= bit;

            if (i < j) {
                std::swap(fftBuffer[static_cast<size_t>(i)], fftBuffer[static_cast<size_t>(j)]);
            }
        }

        // Cooley-Tukey radix-2 FFT.
        for (int length = 2; length <= kFFTSize; length <<= 1) {
            const float angle = (inverse ? 2.0f : -2.0f) * pi / static_cast<float>(length);

            const std::complex<float> wLen = std::polar(1.0f, angle);

            for (int i = 0; i < kFFTSize; i += length) {
                std::complex<float> w(1.0f, 0.0f);

                const int halfLength = length >> 1;

                for (int j = 0; j < halfLength; ++j) {
                    const auto u = fftBuffer[static_cast<size_t>(i + j)];

                    const auto v = fftBuffer[static_cast<size_t>(i + j + halfLength)] * w;

                    fftBuffer[static_cast<size_t>(i + j)] = u + v;

                    fftBuffer[static_cast<size_t>(i + j + halfLength)] = u - v;

                    w *= wLen;
                }
            }
        }

        if (inverse) {
            for (auto& value : fftBuffer)
                value /= static_cast<float>(kFFTSize);
        }
    }

    float magnitudeAtFrequency(float frequency) const {
        if (frequency <= 0.0f)
            return 0.0f;

        const float bin = frequency * static_cast<float>(kFFTSize) / static_cast<float>(sampleRate);

        if (bin < 1.0f || bin >= static_cast<float>(kFFTSize / 2 - 1)) {
            return 0.0f;
        }

        const int lowerBin = static_cast<int>(std::floor(bin));

        const float fraction = bin - static_cast<float>(lowerBin);

        const float a = std::abs(fftBuffer[static_cast<size_t>(lowerBin)]);

        const float b = std::abs(fftBuffer[static_cast<size_t>(lowerBin + 1)]);

        return a + (b - a) * fraction;
    }

    float calculateHarmonicStrength(int midiNote) const {
        const float frequency = 440.0f * std::pow(2.0f, static_cast<float>(midiNote - 69) / 12.0f);

        static constexpr float harmonicWeights[] = {1.0f,  0.65f, 0.40f, 0.25f,
                                                    0.15f, 0.10f, 0.07f, 0.05f};

        float total = 0.0f;

        for (int harmonic = 1; harmonic <= 8; ++harmonic) {
            const float harmonicFrequency = frequency * static_cast<float>(harmonic);

            if (harmonicFrequency >= static_cast<float>(sampleRate * 0.45)) {
                break;
            }

            total += magnitudeAtFrequency(harmonicFrequency) * harmonicWeights[harmonic - 1];
        }

        return total;
    }

    double sampleRate = 44100.0;

    std::array<float, kFFTSize> inputBuffer{};

    std::array<std::complex<float>, kFFTSize> fftBuffer{};

    int writePosition = 0;

    int samplesSinceAnalysis = 0;

    int samplesReceived = 0;

    int samplesAtLastContinuousAnalysis = 0;

    std::array<DetectedNote, kMaxNotes> detectedNotes{};
    int detectedNoteCount = 0;

    float bestFrequencyHz = 0.0f;
    float bestConfidence = 0.0f;
};

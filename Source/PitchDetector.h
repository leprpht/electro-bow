#pragma once

#include <algorithm>
#include <array>
#include <cmath>
#include <cstddef>

#include <aubio.h>
#include <pitch/pitchyin.h>

class PitchDetector
{
public:
    static constexpr int kWindowSize = 4096;
    static constexpr int kHopSize = 1024;

    PitchDetector()
    {
        reset();
    }

    ~PitchDetector()
    {
        destroyAubio();
    }

    PitchDetector(const PitchDetector&) = delete;
    PitchDetector& operator=(const PitchDetector&) = delete;

    void prepare(double newSampleRate)
    {
        sampleRate = std::max(1.0, newSampleRate);

        destroyAubio();
        createAubio();

        reset();
    }

    void push(const float* samples, int numSamples)
    {
        if (samples == nullptr || numSamples <= 0)
            return;

        for (int i = 0; i < numSamples; ++i)
        {
            inputBuffer[
                static_cast<std::size_t>(writePosition)] =
                samples[i];

            writePosition =
                (writePosition + 1) % kWindowSize;

            ++samplesSinceAnalysis;
            ++totalSamples;

            if (totalSamples < kWindowSize)
                continue;

            if (samplesSinceAnalysis >= kHopSize)
            {
                samplesSinceAnalysis = 0;
                analyseCurrentWindow();
            }
        }
    }

    float getFrequencyHz() const noexcept
    {
        return frequencyHz;
    }

    float getConfidence() const noexcept
    {
        return confidence;
    }

    /*
        Compatibility helpers.

        These keep the old debug/UI code working while the
        actual synthesis continues to use continuous Hz.
    */
    float getBestFrequencyHz() const noexcept
    {
        return frequencyHz;
    }

    float getBestConfidence() const noexcept
    {
        return confidence;
    }

    int getMidiNote() const noexcept
    {
        return midiNote;
    }

    void reset() noexcept
    {
        inputBuffer.fill(0.0f);
        analysisBuffer.fill(0.0f);

        writePosition = 0;
        samplesSinceAnalysis = 0;
        totalSamples = 0;

        frequencyHz = 0.0f;
        confidence = 0.0f;
        midiNote = 0;
    }

private:
    void createAubio()
    {
        aubioPitchYin =
            new_aubio_pitchyin(
                static_cast<uint_t>(kWindowSize));

        if (aubioPitchYin == nullptr)
            return;

        /*
            Lower tolerance makes YIN more selective about
            accepting a detected period.
        */
        aubio_pitchyin_set_tolerance(
            aubioPitchYin,
            0.15f);

        aubioInput =
            new_fvec(
                static_cast<uint_t>(kWindowSize));

        aubioOutput =
            new_fvec(1);

        if (aubioInput == nullptr ||
            aubioOutput == nullptr)
        {
            destroyAubio();
        }
    }

    void destroyAubio() noexcept
    {
        if (aubioOutput != nullptr)
        {
            del_fvec(aubioOutput);
            aubioOutput = nullptr;
        }

        if (aubioInput != nullptr)
        {
            del_fvec(aubioInput);
            aubioInput = nullptr;
        }

        if (aubioPitchYin != nullptr)
        {
            del_aubio_pitchyin(aubioPitchYin);
            aubioPitchYin = nullptr;
        }
    }

    void analyseCurrentWindow()
    {
        if (aubioPitchYin == nullptr ||
            aubioInput == nullptr ||
            aubioOutput == nullptr)
        {
            clearDetection();
            return;
        }

        /*
            Reconstruct the current chronological window
            from the circular input buffer.
        */
        for (int i = 0; i < kWindowSize; ++i)
        {
            const int sourceIndex =
                (writePosition + i) % kWindowSize;

            analysisBuffer[
                static_cast<std::size_t>(i)] =
                inputBuffer[
                    static_cast<std::size_t>(sourceIndex)];
        }

        /*
            Reject silence / extremely quiet material before
            giving the buffer to YIN.
        */
        double energy = 0.0;

        for (float sample : analysisBuffer)
        {
            energy +=
                static_cast<double>(sample) *
                static_cast<double>(sample);
        }

        const double rms =
            std::sqrt(
                energy /
                static_cast<double>(kWindowSize));

        if (rms < 0.000001)
        {
            clearDetection();
            return;
        }

        /*
            Copy the analysis window into aubio.
        */
        for (int i = 0; i < kWindowSize; ++i)
        {
            fvec_set_sample(
                aubioInput,
                analysisBuffer[
                    static_cast<std::size_t>(i)],
                static_cast<uint_t>(i));
        }

        aubio_pitchyin_do(
            aubioPitchYin,
            aubioInput,
            aubioOutput);

        const float period =
            fvec_get_sample(
                aubioOutput,
                0);

        const float detectedConfidence =
            std::clamp(
                aubio_pitchyin_get_confidence(
                    aubioPitchYin),
                0.0f,
                1.0f);

        if (period <= 0.0f ||
            !std::isfinite(period) ||
            detectedConfidence <= 0.0f)
        {
            clearDetection();
            return;
        }

        /*
            aubio YIN returns the detected period in samples.
            Convert it directly to Hz.

            No MIDI quantisation happens here.
        */
        const float detectedFrequency =
            static_cast<float>(
                sampleRate /
                static_cast<double>(period));

        if (!std::isfinite(detectedFrequency) ||
            detectedFrequency < 25.0f ||
            detectedFrequency > 1200.0f)
        {
            clearDetection();
            return;
        }

        frequencyHz = detectedFrequency;
        confidence = detectedConfidence;

        /*
            MIDI is only debug information.
            The actual synthesis uses frequencyHz.
        */
        const float midi =
            69.0f +
            12.0f *
            std::log2(
                detectedFrequency /
                440.0f);

        midiNote =
            static_cast<int>(
                std::lround(midi));
    }

    void clearDetection() noexcept
    {
        frequencyHz = 0.0f;
        confidence = 0.0f;
        midiNote = 0;
    }

    double sampleRate = 44100.0;

    std::array<float, kWindowSize>
        inputBuffer{};

    std::array<float, kWindowSize>
        analysisBuffer{};

    int writePosition = 0;
    int samplesSinceAnalysis = 0;
    int totalSamples = 0;

    float frequencyHz = 0.0f;
    float confidence = 0.0f;

    int midiNote = 0;

    aubio_pitchyin_t* aubioPitchYin = nullptr;
    fvec_t* aubioInput = nullptr;
    fvec_t* aubioOutput = nullptr;
};

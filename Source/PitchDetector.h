#pragma once

#include <juce_core/juce_core.h>

#include <algorithm>
#include <atomic>
#include <cmath>
#include <condition_variable>
#include <cstring>
#include <mutex>
#include <thread>
#include <vector>

#include <aubio.h>
#include <pitch/pitchyin.h>

class PitchDetector {
  public:
    static constexpr int kWindowSize = 4096;
    static constexpr int kHopSize = 1024;

    PitchDetector() {
        writeBuffer.resize(kWindowSize, 0.0f);
        analysisBuffer.resize(kWindowSize, 0.0f);

        worker = std::thread(&PitchDetector::workerLoop, this);
    }

    ~PitchDetector() {
        stop();
        destroyAubio();
    }

    PitchDetector(const PitchDetector&) = delete;
    PitchDetector& operator=(const PitchDetector&) = delete;

    void push(const float* samples, int numSamples, double sampleRate) {
        if (samples == nullptr || numSamples <= 0)
            return;

        currentSampleRate.store(sampleRate, std::memory_order_relaxed);

        int offset = 0;

        while (offset < numSamples) {
            const int space = kWindowSize - writePos;
            const int count = std::min(space, numSamples - offset);

            std::memcpy(writeBuffer.data() + writePos, samples + offset,
                        static_cast<size_t>(count) * sizeof(float));

            writePos += count;
            offset += count;

            if (writePos >= kWindowSize) {
                {
                    std::lock_guard<std::mutex> lock(bufferMutex);

                    std::swap(writeBuffer, analysisBuffer);
                    hasNewData = true;
                }

                writePos = 0;
                cv.notify_one();
            }
        }
    }

    float getFrequencyHz() const noexcept {
        return frequency.load(std::memory_order_relaxed);
    }

    float getConfidence() const noexcept {
        return confidence.load(std::memory_order_relaxed);
    }

    int getMidiNote() const noexcept {
        return midiNote.load(std::memory_order_relaxed);
    }

    void reset() noexcept {
        std::fill(writeBuffer.begin(), writeBuffer.end(), 0.0f);
        std::fill(analysisBuffer.begin(), analysisBuffer.end(), 0.0f);

        writePos = 0;

        frequency.store(0.0f, std::memory_order_relaxed);
        confidence.store(0.0f, std::memory_order_relaxed);
        midiNote.store(0, std::memory_order_relaxed);
    }

  private:
    void createAubio() {
        destroyAubio();

        aubioPitchYin = new_aubio_pitchyin(static_cast<uint_t>(kWindowSize));

        if (aubioPitchYin == nullptr)
            return;

        aubio_pitchyin_set_tolerance(aubioPitchYin, 0.15f);

        aubioInput = new_fvec(static_cast<uint_t>(kWindowSize));

        aubioOutput = new_fvec(1);

        if (aubioInput == nullptr || aubioOutput == nullptr) {
            destroyAubio();
        }
    }

    void destroyAubio() noexcept {
        if (aubioOutput != nullptr) {
            del_fvec(aubioOutput);
            aubioOutput = nullptr;
        }

        if (aubioInput != nullptr) {
            del_fvec(aubioInput);
            aubioInput = nullptr;
        }

        if (aubioPitchYin != nullptr) {
            del_aubio_pitchyin(aubioPitchYin);
            aubioPitchYin = nullptr;
        }
    }

    void workerLoop() {
        double lastSampleRate = 0.0;

        while (true) {
            {
                std::unique_lock<std::mutex> lock(bufferMutex);

                cv.wait(lock, [this] { return hasNewData || shouldStop; });

                if (shouldStop)
                    break;

                hasNewData = false;
            }

            const double sampleRate = currentSampleRate.load(std::memory_order_relaxed);

            if (sampleRate <= 0.0)
                continue;

            if (aubioPitchYin == nullptr || std::abs(sampleRate - lastSampleRate) > 0.5) {
                createAubio();
                lastSampleRate = sampleRate;
            }

            if (aubioPitchYin == nullptr || aubioInput == nullptr || aubioOutput == nullptr) {
                frequency.store(0.0f, std::memory_order_relaxed);

                confidence.store(0.0f, std::memory_order_relaxed);

                midiNote.store(0, std::memory_order_relaxed);

                continue;
            }

            /*
                aubio_pitchyin_do() expects one complete analysis
                frame. We therefore analyse the complete 4096-sample
                window at once.
            */

            for (int i = 0; i < kWindowSize; ++i) {
                fvec_set_sample(aubioInput, analysisBuffer[static_cast<size_t>(i)],
                                static_cast<uint_t>(i));
            }

            aubio_pitchyin_do(aubioPitchYin, aubioInput, aubioOutput);

            const float period = fvec_get_sample(aubioOutput, 0);

            const float candidateConfidence = aubio_pitchyin_get_confidence(aubioPitchYin);
            DBG("YIN period = " + juce::String(period) +
                "  freq = " + juce::String(sampleRate / period) +
                "  conf = " + juce::String(candidateConfidence));

            float detectedFrequency = 0.0f;

            if (period > 0.0f && std::isfinite(period)) {
                detectedFrequency = static_cast<float>(sampleRate / period);
            }

            const float detectedConfidence = std::clamp(candidateConfidence, 0.0f, 1.0f);

            if (detectedFrequency >= 25.0f && detectedFrequency <= 1200.0f &&
                detectedConfidence > 0.0f) {
                const float midi = 69.0f + 12.0f * std::log2(detectedFrequency / 440.0f);

                const int roundedMidi = static_cast<int>(std::lround(midi));

                frequency.store(detectedFrequency, std::memory_order_relaxed);

                confidence.store(detectedConfidence, std::memory_order_relaxed);

                midiNote.store(roundedMidi, std::memory_order_relaxed);
            } else {
                frequency.store(0.0f, std::memory_order_relaxed);

                confidence.store(0.0f, std::memory_order_relaxed);

                midiNote.store(0, std::memory_order_relaxed);
            }
        }
    }

    void stop() noexcept {
        {
            std::lock_guard<std::mutex> lock(bufferMutex);
            shouldStop = true;
        }

        cv.notify_one();

        if (worker.joinable())
            worker.join();
    }

    std::vector<float> writeBuffer;
    std::vector<float> analysisBuffer;

    int writePos = 0;

    std::thread worker;

    std::mutex bufferMutex;
    std::condition_variable cv;

    bool hasNewData = false;
    bool shouldStop = false;

    std::atomic<double> currentSampleRate{44100.0};

    std::atomic<float> frequency{0.0f};
    std::atomic<float> confidence{0.0f};
    std::atomic<int> midiNote{0};

    aubio_pitchyin_t* aubioPitchYin = nullptr;

    fvec_t* aubioInput = nullptr;
    fvec_t* aubioOutput = nullptr;
};

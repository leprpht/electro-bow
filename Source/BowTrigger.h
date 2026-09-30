#pragma once

#include <algorithm>
#include <cmath>

class BowTrigger {
  public:
    void prepare(double newSampleRate) {
        sampleRate = std::max(1.0, newSampleRate);
        reset();
    }

    void reset() {
        envelope = 0.0f;
        wasAboveThreshold = false;
        releaseCounter = 0;
        justTriggered = false;
        justReleased = false;
    }

    bool processSample(float input) {
        justTriggered = false;
        justReleased = false;

        const float inputLevel = std::abs(input);

        const float coefficient = inputLevel > envelope ? timeToCoefficient(detectorAttackMs)
                                                        : timeToCoefficient(detectorReleaseMs);

        envelope += (inputLevel - envelope) * coefficient;

        const float level = std::clamp(envelope * sensitivity, 0.0f, 1.0f);

        if (!wasAboveThreshold && level >= triggerThreshold) {
            wasAboveThreshold = true;
            releaseCounter = 0;
            justTriggered = true;
        }

        if (wasAboveThreshold && level <= releaseThreshold) {
            ++releaseCounter;

            if (releaseCounter >= releaseSamplesRequired) {
                wasAboveThreshold = false;
                releaseCounter = 0;
                justReleased = true;
            }
        } else if (level > releaseThreshold) {
            releaseCounter = 0;
        }

        return justTriggered;
    }

    bool consumeRelease() {
        const bool result = justReleased;
        justReleased = false;
        return result;
    }

    float getLevel() const noexcept {
        return std::clamp(envelope * sensitivity, 0.0f, 1.0f);
    }

    bool isNoteActive() const noexcept {
        return wasAboveThreshold;
    }

  private:
    float timeToCoefficient(float milliseconds) const noexcept {
        if (milliseconds <= 0.0f)
            return 1.0f;

        const double seconds = static_cast<double>(milliseconds) * 0.001;

        return static_cast<float>(1.0 - std::exp(-1.0 / (seconds * sampleRate)));
    }

    double sampleRate = 44100.0;

    float sensitivity = 1.0f;

    float triggerThreshold = 0.03f;
    float releaseThreshold = 0.015f;

    float detectorAttackMs = 2.0f;
    float detectorReleaseMs = 30.0f;

    float envelope = 0.0f;

    bool wasAboveThreshold = false;

    bool justTriggered = false;
    bool justReleased = false;

    int releaseCounter = 0;

    static constexpr int releaseSamplesRequired = 256;
};

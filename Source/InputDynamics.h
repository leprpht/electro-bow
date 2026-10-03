#pragma once

#include <algorithm>
#include <cmath>

// Continuous input-level follower used for musical intensity.  This is
// deliberately separate from BowTrigger: crossing a trigger threshold is a
// note-state event, while this value remains meaningful throughout a note.
class InputDynamics {
  public:
    void prepare(double newSampleRate) {
        sampleRate = std::max(1.0, newSampleRate);
        reset();
    }

    void reset() noexcept {
        level = 0.0f;
    }

    float processSample(float input) noexcept {
        const float magnitude =
            std::isfinite(input) ? std::clamp(std::abs(input), 0.0f, 1.0f) : 0.0f;
        const float coefficient =
            magnitude > level ? timeToCoefficient(attackMs) : timeToCoefficient(releaseMs);

        level += (magnitude - level) * coefficient;
        return level;
    }

    float getLevel() const noexcept {
        return level;
    }

  private:
    float timeToCoefficient(float milliseconds) const noexcept {
        if (milliseconds <= 0.0f)
            return 1.0f;

        const double seconds = static_cast<double>(milliseconds) * 0.001;
        return static_cast<float>(1.0 - std::exp(-1.0 / (seconds * sampleRate)));
    }

    double sampleRate = 44100.0;
    float level = 0.0f;

    static constexpr float attackMs = 3.0f;
    static constexpr float releaseMs = 60.0f;
};

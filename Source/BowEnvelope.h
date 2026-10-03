#pragma once

#include <algorithm>
#include <cmath>

class BowEnvelope {
  public:
    void prepare(double newSampleRate) {
        sampleRate = std::max(1.0, newSampleRate);
        reset();
    }

    void reset() {
        state = State::Idle;
        level = 0.0f;
        targetLevel = 0.0f;
    }

    void setAttackMs(float newAttackMs) {
        attackMs = std::max(0.0f, newAttackMs);
    }

    void setNaturalResonanceMs(float newNaturalResonanceMs) {
        naturalResonanceMs = std::max(0.0f, newNaturalResonanceMs);
    }

    void trigger(float inputLevel) {
        targetLevel = std::clamp(inputLevel, 0.0f, 1.0f);

        if (targetLevel <= 0.0001f)
            targetLevel = 0.0001f;

        state = State::Attack;
    }

    void sustain() {
        if (state != State::Idle)
            state = State::Sustain;
    }

    void setSustainLevel(float newLevel) {
        targetLevel = std::clamp(newLevel, 0.0001f, 1.0f);

        if (state == State::Sustain)
            level = targetLevel;
    }

    void release() {
        if (state != State::Idle)
            state = State::NaturalResonance;
    }

    float process() {
        switch (state) {
        case State::Idle:
            level = 0.0f;
            break;

        case State::Attack: {
            const float coefficient = timeToCoefficient(attackMs);

            level += (targetLevel - level) * coefficient;

            if (attackMs <= 0.0f || std::abs(targetLevel - level) < 0.0001f) {
                level = targetLevel;
                state = State::Sustain;
            }

            break;
        }

        case State::Sustain:
            level = targetLevel;
            break;

        case State::NaturalResonance: {
            const float coefficient = timeToCoefficient(naturalResonanceMs);

            level += (0.0f - level) * coefficient;

            if (naturalResonanceMs <= 0.0f || level < 0.0001f) {
                level = 0.0f;
                state = State::Idle;
            }

            break;
        }
        }

        return level;
    }

    float getLevel() const noexcept {
        return level;
    }

    bool isActive() const noexcept {
        return state != State::Idle;
    }

    bool isReleasing() const noexcept {
        return state == State::NaturalResonance;
    }

  private:
    enum class State { Idle, Attack, Sustain, NaturalResonance };

    float timeToCoefficient(float milliseconds) const noexcept {
        if (milliseconds <= 0.0f)
            return 1.0f;

        const double seconds = static_cast<double>(milliseconds) * 0.001;

        return static_cast<float>(1.0 - std::exp(-1.0 / (seconds * sampleRate)));
    }

    double sampleRate = 44100.0;

    float attackMs = 50.0f;
    float naturalResonanceMs = 50.0f;

    float level = 0.0f;
    float targetLevel = 0.0f;

    State state = State::Idle;
};

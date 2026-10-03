#pragma once

#include <algorithm>
#include <cmath>

class BowTrigger {
  public:
    // Detects physical attack/release state from the input amplitude. This is
    // intentionally separate from spectral voice detection: harmonics and
    // pitch confidence may change during one physical pluck.
    void prepare(double newSampleRate) {
        sampleRate = std::max(1.0, newSampleRate);
        reset();
    }

    void reset() {
        envelope = 0.0f;
        onsetBaseline = 0.0f;
        wasAboveThreshold = false;
        releaseCounter = 0;
        retriggerCooldown = 0;
        retriggerArmed = true;
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

        // Keep a deliberately slower reference level for detecting a new
        // physical pluck while another string is still audible.  The normal
        // threshold edge below handles the first attack; this differential
        // path is only for a later, distinct rise above an existing sustain.
        onsetBaseline += (level - onsetBaseline) * timeToCoefficient(onsetBaselineMs);

        if (retriggerCooldown > 0)
            --retriggerCooldown;

        if (!retriggerArmed && level - onsetBaseline < retriggerRearmThreshold)
            retriggerArmed = true;

        if (!wasAboveThreshold && level >= triggerThreshold) {
            wasAboveThreshold = true;
            releaseCounter = 0;
            justTriggered = true;
            retriggerCooldown = retriggerCooldownSamples();
            retriggerArmed = false;
        } else if (wasAboveThreshold && retriggerArmed && retriggerCooldown == 0 &&
                   level >= triggerThreshold && level - onsetBaseline >= retriggerRiseThreshold) {
            // Do not require every sounding string to decay to silence
            // before allowing another note in a chord or repeated pluck.
            // The cooldown turns the rise of one strong transient into one
            // event rather than a sequence of duplicate attacks.
            justTriggered = true;
            retriggerCooldown = retriggerCooldownSamples();
            retriggerArmed = false;
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
        // The current processor uses analyzer disappearance to begin natural
        // voice release; this accessor remains available for a future policy
        // that wants to combine physical release with analyzer state.
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
    float onsetBaselineMs = 80.0f;

    float envelope = 0.0f;
    float onsetBaseline = 0.0f;

    bool wasAboveThreshold = false;

    bool justTriggered = false;
    bool justReleased = false;

    int releaseCounter = 0;
    int retriggerCooldown = 0;
    bool retriggerArmed = true;

    static constexpr int releaseSamplesRequired = 256;
    static constexpr float retriggerRiseThreshold = 0.035f;
    static constexpr float retriggerRearmThreshold = 0.015f;
    static constexpr float retriggerCooldownMs = 30.0f;

    int retriggerCooldownSamples() const noexcept {
        return std::max(1, static_cast<int>(std::lround(
                               sampleRate * static_cast<double>(retriggerCooldownMs) * 0.001)));
    }
};

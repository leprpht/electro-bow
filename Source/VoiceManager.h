#pragma once

#include "BowEnvelope.h"
#include "PolyphonicAnalyzer.h"

#include <Bowed.h>

#include <algorithm>
#include <array>
#include <cmath>
#include <cstdint>

class VoiceManager {
  public:
    static constexpr int kMaxVoices = PolyphonicAnalyzer::kMaxVoices;

    struct Voice {
        stk::Bowed bowed;
        BowEnvelope envelope;

        int analyzerVoiceId = -1;
        float frequencyHz = 0.0f;
        float strength = 0.0f;

        int lostFrames = 0;

        bool active = false;
        bool releasing = false;
    };

    void prepare(double newSampleRate, float newAttackMs, float newNaturalResonanceMs) {
        sampleRate = std::max(1.0, newSampleRate);
        attackMs = std::max(0.0f, newAttackMs);
        naturalResonanceMs = std::max(0.0f, newNaturalResonanceMs);

        stk::Stk::setSampleRate(sampleRate);

        for (int i = 0; i < kMaxVoices; ++i) {
            Voice& voice = voices[static_cast<std::size_t>(i)];

            voice.bowed.clear();

            voice.envelope.prepare(sampleRate);
            voice.envelope.setAttackMs(attackMs);
            voice.envelope.setNaturalResonanceMs(naturalResonanceMs);

            resetVoice(voice);
        }

        lastAnalysisGeneration = 0;
    }

    void reset() {
        for (int i = 0; i < kMaxVoices; ++i) {
            Voice& voice = voices[static_cast<std::size_t>(i)];

            voice.bowed.clear();
            voice.envelope.reset();

            resetVoice(voice);
        }

        lastAnalysisGeneration = 0;
    }

    void updateDetectedVoices(const PolyphonicAnalyzer& analyzer, bool allowNewVoices = true) {
        const auto analysisGeneration = analyzer.getAnalysisGeneration();

        // PolyphonicAnalyzer is fed in host-sized blocks, but it only
        // publishes a new voice set on FFT hops. Processing the same set more
        // than once would consume the loss grace period in host-block time
        // and could release a voice during a single transient.
        if (analysisGeneration == lastAnalysisGeneration)
            return;

        lastAnalysisGeneration = analysisGeneration;

        if (!analyzer.isAnalysisReliable()) {
            markAllVoicesLost();
            return;
        }

        std::array<bool, kMaxVoices> matched{};

        const auto& detectedVoices = analyzer.getVoices();

        for (const auto& detected : detectedVoices) {
            if (!detected.active)
                continue;

            if (detected.id < 0)
                continue;

            if (detected.trackedFrequencyHz <= 0.0f)
                continue;

            float confidence = detected.trackerConfidence;

            if (confidence < 0.0f)
                confidence = 0.0f;

            if (confidence > 1.0f)
                confidence = 1.0f;

            float strength = detected.strength;

            if (!std::isfinite(strength) || strength <= 0.0f)
                strength = confidence;

            strength = limit01(strength);

            int voiceIndex = findVoiceByAnalyzerId(detected.id, matched);

            if (voiceIndex < 0) {
                voiceIndex = findVoiceByFrequency(detected.trackedFrequencyHz, matched);
            }

            // New STK attacks are permitted only for a physical note-on. If
            // the analyzer changes candidates during an existing pluck,
            // continuity handling below must update/reuse a live voice.
            if (voiceIndex < 0 && allowNewVoices) {
                voiceIndex = findFreeVoice();

                if (voiceIndex < 0)
                    voiceIndex = findReleasingVoice();
            }

            if (voiceIndex < 0 && !allowNewVoices) {
                // Once the physical attack has been consumed, an unmatched
                // spectral candidate is a retuning of an existing note until
                // proven otherwise. Reusing the nearest live voice keeps a
                // pitch jump from becoming a second Bowed attack. A genuinely
                // new note remains ignored here because it has no note-on.
                voiceIndex = findNearestVoiceForContinuity(detected.trackedFrequencyHz, matched);
            }

            if (voiceIndex < 0)
                continue;

            Voice& voice = voices[static_cast<std::size_t>(voiceIndex)];

            if (voice.releasing) {
                if (inputLevel < 0.05f)
                    continue;

                voiceIndex = -1;

                if (allowNewVoices)
                    voiceIndex = findFreeVoice();

                if (voiceIndex < 0 && allowNewVoices)
                    voiceIndex = findReleasingVoice();

                if (voiceIndex < 0)
                    continue;
            }

            Voice& selectedVoice = voices[static_cast<std::size_t>(voiceIndex)];

            if (!selectedVoice.active || selectedVoice.releasing) {
                startVoice(selectedVoice, detected.id, detected.trackedFrequencyHz, strength);
            } else {
                updateExistingVoice(selectedVoice, detected.id, detected.trackedFrequencyHz,
                                    strength);
            }

            matched[static_cast<std::size_t>(voiceIndex)] = true;
        }

        for (int i = 0; i < kMaxVoices; ++i) {
            if (matched[static_cast<std::size_t>(i)])
                continue;

            Voice& voice = voices[static_cast<std::size_t>(i)];

            if (!voice.active)
                continue;

            if (voice.releasing)
                continue;

            ++voice.lostFrames;

            if (voice.lostFrames >= kLostFramesBeforeRelease) {
                beginRelease(voice);
            }
        }
    }

    void releaseAll() {
        for (auto& voice : voices)
            beginRelease(voice);
    }

    void setInputLevel(float newInputLevel) {
        inputLevel = limit01(newInputLevel);

        for (auto& voice : voices) {
            if (!voice.active || voice.releasing)
                continue;

            voice.envelope.setSustainLevel(voice.strength * (0.2f + 0.8f * inputLevel));

            applyBowParameters(voice);
        }
    }

    void setBowParameters(float newBowPressure, float newBowSpeed, float newFriction) {
        bowPressure = limit01(newBowPressure);
        bowSpeed = limit01(newBowSpeed);
        friction = limit01(newFriction);

        for (int i = 0; i < kMaxVoices; ++i) {
            Voice& voice = voices[static_cast<std::size_t>(i)];

            if (voice.active)
                applyBowParameters(voice);
        }
    }

    void setEnvelopeParameters(float newAttackMs, float newNaturalResonanceMs) {
        attackMs = std::max(0.0f, newAttackMs);

        naturalResonanceMs = std::max(0.0f, newNaturalResonanceMs);

        for (int i = 0; i < kMaxVoices; ++i) {
            Voice& voice = voices[static_cast<std::size_t>(i)];

            voice.envelope.setAttackMs(attackMs);

            voice.envelope.setNaturalResonanceMs(naturalResonanceMs);
        }
    }

    float processSample() {
        float mixedOutput = 0.0f;

        for (int i = 0; i < kMaxVoices; ++i) {
            Voice& voice = voices[static_cast<std::size_t>(i)];

            if (!voice.active)
                continue;

            const float envelopeLevel = voice.envelope.process();

            if (envelopeLevel > 0.000001f || !voice.releasing) {
                const float bowedSample = static_cast<float>(voice.bowed.tick());

                mixedOutput += bowedSample * envelopeLevel;
            }

            if (voice.releasing && !voice.envelope.isActive()) {
                voice.bowed.clear();
                resetVoice(voice);
            }
        }

        if (mixedOutput > 1.0f)
            mixedOutput = 1.0f;

        if (mixedOutput < -1.0f)
            mixedOutput = -1.0f;

        return mixedOutput;
    }

    int getActiveVoiceCount() const noexcept {
        int count = 0;

        for (int i = 0; i < kMaxVoices; ++i) {
            if (voices[static_cast<std::size_t>(i)].active) {
                ++count;
            }
        }

        return count;
    }

    float getVoiceFrequencyHz(int index) const noexcept {
        if (index < 0 || index >= kMaxVoices)
            return 0.0f;

        return voices[static_cast<std::size_t>(index)].frequencyHz;
    }

    float getVoiceStrength(int index) const noexcept {
        if (index < 0 || index >= kMaxVoices)
            return 0.0f;

        return voices[static_cast<std::size_t>(index)].strength;
    }

    int getVoiceAnalyzerId(int index) const noexcept {
        if (index < 0 || index >= kMaxVoices)
            return -1;

        return voices[static_cast<std::size_t>(index)].analyzerVoiceId;
    }

    bool isVoiceActive(int index) const noexcept {
        if (index < 0 || index >= kMaxVoices)
            return false;

        return voices[static_cast<std::size_t>(index)].active;
    }

  private:
    static constexpr int kLostFramesBeforeRelease = 3;
    static constexpr float kFrequencyMatchCents = 100.0f;

    static float limit01(float value) noexcept {
        if (value < 0.0f)
            return 0.0f;

        if (value > 1.0f)
            return 1.0f;

        return value;
    }

    int findFreeVoice() const noexcept {
        for (int i = 0; i < kMaxVoices; ++i) {
            const Voice& voice = voices[static_cast<std::size_t>(i)];

            if (!voice.active)
                return i;
        }

        return -1;
    }

    void markAllVoicesLost() {
        for (auto& voice : voices) {
            if (!voice.active || voice.releasing)
                continue;

            ++voice.lostFrames;

            if (voice.lostFrames >= kLostFramesBeforeRelease)
                beginRelease(voice);
        }
    }

    int findReleasingVoice() const noexcept {
        int bestIndex = -1;
        float lowestLevel = 2.0f;

        for (int i = 0; i < kMaxVoices; ++i) {
            const Voice& voice = voices[static_cast<std::size_t>(i)];

            if (!voice.active || !voice.releasing)
                continue;

            if (voice.envelope.getLevel() < lowestLevel) {
                lowestLevel = voice.envelope.getLevel();
                bestIndex = i;
            }
        }

        return bestIndex;
    }

    int findVoiceByAnalyzerId(int analyzerId,
                              const std::array<bool, kMaxVoices>& matched) const noexcept {
        for (int i = 0; i < kMaxVoices; ++i) {
            if (matched[static_cast<std::size_t>(i)]) {
                continue;
            }

            const Voice& voice = voices[static_cast<std::size_t>(i)];

            if (!voice.active)
                continue;

            if (voice.analyzerVoiceId == analyzerId)
                return i;
        }

        return -1;
    }

    int findVoiceByFrequency(float frequency,
                             const std::array<bool, kMaxVoices>& matched) const noexcept {
        if (frequency <= 0.0f || !std::isfinite(frequency)) {
            return -1;
        }

        int bestIndex = -1;
        float bestCents = kFrequencyMatchCents;

        for (int i = 0; i < kMaxVoices; ++i) {
            if (matched[static_cast<std::size_t>(i)]) {
                continue;
            }

            const Voice& voice = voices[static_cast<std::size_t>(i)];

            if (!voice.active)
                continue;

            if (voice.releasing)
                continue;

            if (voice.frequencyHz <= 0.0f)
                continue;

            const float ratio = frequency / voice.frequencyHz;

            if (ratio <= 0.0f || !std::isfinite(ratio)) {
                continue;
            }

            const float cents = std::abs(1200.0f * std::log2(ratio));

            if (cents < bestCents) {
                bestCents = cents;
                bestIndex = i;
            }
        }

        return bestIndex;
    }

    int findNearestVoiceForContinuity(float frequency,
                                      const std::array<bool, kMaxVoices>& matched) const noexcept {
        if (frequency <= 0.0f || !std::isfinite(frequency))
            return -1;

        int bestIndex = -1;
        float bestCents = 100000.0f;

        for (int i = 0; i < kMaxVoices; ++i) {
            if (matched[static_cast<std::size_t>(i)]) {
                continue;
            }

            const Voice& voice = voices[static_cast<std::size_t>(i)];

            if (!voice.active || voice.releasing || voice.frequencyHz <= 0.0f) {
                continue;
            }

            // This function is only reached when no physical note-on was
            // detected. There is therefore no new-note candidate to protect
            // with a proximity threshold; the closest unmatched live voice
            // is the continuity target.
            const float cents = std::abs(1200.0f * std::log2(frequency / voice.frequencyHz));

            if (cents < bestCents) {
                bestCents = cents;
                bestIndex = i;
            }
        }

        return bestIndex;
    }

    void startVoice(Voice& voice, int analyzerId, float frequency, float strength) {
        // This is the sole synthesis note-on path. Pitch changes must use
        // updateExistingVoice() so they do not restart the Bowed attack.
        voice.bowed.clear();

        voice.bowed.setFrequency(static_cast<stk::StkFloat>(frequency));

        applyBowParameters(voice);

        float amplitude = 0.05f + strength * 0.95f;
        amplitude *= 0.2f + 0.8f * inputLevel;

        amplitude = std::max(0.05f, amplitude);

        amplitude = std::min(1.0f, amplitude);

        const double attackSeconds = std::max(0.0001, static_cast<double>(attackMs) * 0.001);

        double attackRate = 0.005 / attackSeconds;

        attackRate = std::max(0.0001, attackRate);

        attackRate = std::min(100.0, attackRate);

        voice.bowed.startBowing(static_cast<stk::StkFloat>(amplitude),
                                static_cast<stk::StkFloat>(attackRate));

        voice.envelope.reset();
        voice.envelope.setAttackMs(attackMs);
        voice.envelope.setNaturalResonanceMs(naturalResonanceMs);
        voice.envelope.trigger(strength * (0.2f + 0.8f * inputLevel));

        voice.analyzerVoiceId = analyzerId;
        voice.frequencyHz = frequency;
        voice.strength = strength;

        voice.lostFrames = 0;
        voice.active = true;
        voice.releasing = false;
    }

    void updateExistingVoice(Voice& voice, int analyzerId, float frequency, float strength) {
        // Analyzer IDs and spectral candidates can change as a pluck decays;
        // updating frequency here preserves one physical attack as one STK
        // voice.
        if (frequency > 0.0f && std::isfinite(frequency)) {
            voice.frequencyHz = frequency;

            voice.bowed.setFrequency(static_cast<stk::StkFloat>(frequency));
        }

        voice.analyzerVoiceId = analyzerId;
        voice.strength = strength;
        voice.lostFrames = 0;
        voice.releasing = false;

        voice.envelope.sustain();
        voice.envelope.setSustainLevel(strength * (0.2f + 0.8f * inputLevel));
    }

    void beginRelease(Voice& voice) {
        if (!voice.active)
            return;

        if (voice.releasing)
            return;

        voice.releasing = true;
        voice.lostFrames = 0;

        voice.envelope.release();
    }

    void applyBowParameters(Voice& voice) {
        const float dynamicPressure = bowPressure * (0.2f + 0.8f * inputLevel);
        const float dynamicSpeed = bowSpeed * (0.5f + 0.5f * inputLevel);

        voice.bowed.controlChange(2, static_cast<stk::StkFloat>(dynamicPressure * 128.0f));

        voice.bowed.controlChange(4, static_cast<stk::StkFloat>(friction * 128.0f));

        voice.bowed.controlChange(100, static_cast<stk::StkFloat>(dynamicSpeed * 128.0f));
    }

    void resetVoice(Voice& voice) {
        voice.analyzerVoiceId = -1;
        voice.frequencyHz = 0.0f;
        voice.strength = 0.0f;
        voice.lostFrames = 0;
        voice.active = false;
        voice.releasing = false;
    }

    double sampleRate = 44100.0;

    float bowPressure = 0.5f;
    float bowSpeed = 0.5f;
    float friction = 0.127f;

    float attackMs = 50.0f;
    float naturalResonanceMs = 50.0f;
    float inputLevel = 0.0f;

    std::array<Voice, kMaxVoices> voices{};
    std::uint64_t lastAnalysisGeneration = 0;
};

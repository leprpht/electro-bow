#pragma once

#include "BowEnvelope.h"
#include "PolyPitchDetector.h"
#include "PolyphonicAnalyzer.h"

#include <Bowed.h>

#include <algorithm>
#include <array>
#include <cmath>

class VoiceManager {
  public:
    static constexpr int kMaxVoices = 8;

    struct Voice {
        stk::Bowed bowed;
        BowEnvelope envelope;

        int midiNote = -1;
        float frequencyHz = 0.0f;

        float strength = 0.0f;

        bool active = false;
        bool releasing = false;
    };

    void prepare(double newSampleRate, float newAttackMs, float newReleaseMs) {
        sampleRate = std::max(1.0, newSampleRate);

        stk::Stk::setSampleRate(sampleRate);

        for (auto& voice : voices) {
            voice.bowed.clear();

            voice.envelope.prepare(sampleRate);
            voice.envelope.setAttackMs(newAttackMs);
            voice.envelope.setReleaseMs(newReleaseMs);

            voice.midiNote = -1;
            voice.frequencyHz = 0.0f;
            voice.strength = 0.0f;
            voice.active = false;
            voice.releasing = false;
        }
    }

    void reset() {
        for (auto& voice : voices) {
            if (voice.active)
                voice.bowed.stopBowing(0.005);

            voice.bowed.clear();
            voice.envelope.reset();

            voice.midiNote = -1;
            voice.frequencyHz = 0.0f;
            voice.strength = 0.0f;
            voice.active = false;
            voice.releasing = false;
        }
    }

    void updateDetectedVoices(const PolyphonicAnalyzer& analyzer) {
        std::array<bool, kMaxVoices> matched{};
        for (const auto& detected : analyzer.getVoices()) {
            if (detected.trackedFrequencyHz <= 0.0f)
                continue;
            int best = -1;
            float bestCents = 100000.0f;
            for (int i = 0; i < kMaxVoices; ++i) {
                const auto& voice = voices[static_cast<size_t>(i)];
                if (!voice.active || matched[static_cast<size_t>(i)] || voice.frequencyHz <= 0.0f)
                    continue;
                const float cents =
                    std::abs(1200.0f * std::log2(detected.trackedFrequencyHz / voice.frequencyHz));
                if (cents < 100.0f && cents < bestCents) {
                    best = i;
                    bestCents = cents;
                }
            }
            if (best < 0)
                best = findFreeVoice();
            if (best < 0)
                continue;
            auto& voice = voices[static_cast<size_t>(best)];
            if (!voice.active)
                startVoice(voice, detected.trackedFrequencyHz, detected.strength);
            else {
                voice.frequencyHz = detected.trackedFrequencyHz;
                voice.bowed.setFrequency(static_cast<stk::StkFloat>(voice.frequencyHz));
                voice.strength = detected.strength;
                voice.releasing = false;
                voice.envelope.sustain();
            }
            matched[static_cast<size_t>(best)] = true;
        }
        for (int i = 0; i < kMaxVoices; ++i) {
            auto& voice = voices[static_cast<size_t>(i)];
            if (voice.active && !matched[static_cast<size_t>(i)] && !voice.releasing) {
                voice.releasing = true;
                voice.envelope.release();
                voice.bowed.stopBowing(0.005);
            }
        }
    }

    void setBowParameters(float newBowPressure, float newBowSpeed, float newFriction) {
        bowPressure = std::clamp(newBowPressure, 0.0f, 1.0f);

        bowSpeed = std::clamp(newBowSpeed, 0.0f, 1.0f);

        friction = std::clamp(newFriction, 0.0f, 1.0f);
    }

    void setEnvelopeParameters(float newAttackMs, float newReleaseMs) {
        attackMs = std::max(0.0f, newAttackMs);

        releaseMs = std::max(0.0f, newReleaseMs);

        for (auto& voice : voices) {
            voice.envelope.setAttackMs(attackMs);
            voice.envelope.setReleaseMs(releaseMs);
        }
    }

    void updateDetectedNotes(const PolyPitchDetector& detector) {
        std::array<bool, kMaxVoices> matchedVoices{};

        // ------------------------------------------------------------------
        // First: keep voices which are still present.
        //
        // This is what lets a common note survive a chord change without
        // being retriggered.
        // ------------------------------------------------------------------

        for (int noteIndex = 0; noteIndex < detector.getNumNotes(); ++noteIndex) {
            const auto detected = detector.getNote(noteIndex);

            for (int voiceIndex = 0; voiceIndex < kMaxVoices; ++voiceIndex) {
                auto& voice = voices[static_cast<size_t>(voiceIndex)];

                if (!voice.active)
                    continue;

                if (matchedVoices[static_cast<size_t>(voiceIndex)]) {
                    continue;
                }

                if (voice.midiNote == detected.midiNote) {
                    voice.strength = detected.strength;

                    voice.releasing = false;

                    voice.envelope.sustain();

                    matchedVoices[static_cast<size_t>(voiceIndex)] = true;

                    break;
                }
            }
        }

        // ------------------------------------------------------------------
        // Second: notes which are no longer detected begin release.
        // ------------------------------------------------------------------

        for (int voiceIndex = 0; voiceIndex < kMaxVoices; ++voiceIndex) {
            auto& voice = voices[static_cast<size_t>(voiceIndex)];

            if (!voice.active)
                continue;

            if (matchedVoices[static_cast<size_t>(voiceIndex)]) {
                continue;
            }

            if (!voice.releasing) {
                voice.releasing = true;
                voice.envelope.release();
                voice.bowed.stopBowing(0.005);
            }
        }

        // ------------------------------------------------------------------
        // Third: create voices for newly detected notes.
        // ------------------------------------------------------------------

        for (int noteIndex = 0; noteIndex < detector.getNumNotes(); ++noteIndex) {
            const auto detected = detector.getNote(noteIndex);

            bool alreadyExists = false;

            for (int voiceIndex = 0; voiceIndex < kMaxVoices; ++voiceIndex) {
                const auto& voice = voices[static_cast<size_t>(voiceIndex)];

                if (voice.active && !voice.releasing && voice.midiNote == detected.midiNote) {
                    alreadyExists = true;
                    break;
                }
            }

            if (alreadyExists)
                continue;

            const int voiceIndex = findFreeVoice();

            if (voiceIndex < 0)
                continue;

            startVoice(voices[static_cast<size_t>(voiceIndex)], detected);
        }
    }

    float processSample() {
        float mixedOutput = 0.0f;

        for (auto& voice : voices) {
            if (!voice.active)
                continue;

            const float envelope = voice.envelope.process();

            if (envelope > 0.000001f || !voice.releasing) {
                const float bowedSample = static_cast<float>(voice.bowed.tick());

                mixedOutput += bowedSample * envelope;
            }

            if (voice.releasing && !voice.envelope.isActive()) {
                voice.bowed.clear();

                voice.midiNote = -1;
                voice.frequencyHz = 0.0f;
                voice.strength = 0.0f;
                voice.active = false;
                voice.releasing = false;
            }
        }

        return mixedOutput;
    }

    int getActiveVoiceCount() const noexcept {
        int count = 0;

        for (const auto& voice : voices) {
            if (voice.active)
                ++count;
        }

        return count;
    }

    int getVoiceMidiNote(int index) const noexcept {
        if (index < 0 || index >= kMaxVoices) {
            return -1;
        }

        return voices[static_cast<size_t>(index)].midiNote;
    }

  private:
    int findFreeVoice() const noexcept {
        for (int i = 0; i < kMaxVoices; ++i) {
            const auto& voice = voices[static_cast<size_t>(i)];

            if (!voice.active)
                return i;
        }

        return -1;
    }

    void startVoice(Voice& voice, const PolyPitchDetector::DetectedNote& note) {
        const float frequency =
            440.0f * std::pow(2.0f, static_cast<float>(note.midiNote - 69) / 12.0f);

        startVoice(voice, frequency, note.strength);
        voice.midiNote = note.midiNote;
    }

    void startVoice(Voice& voice, float frequency, float strength) {

        voice.bowed.clear();

        voice.bowed.setFrequency(static_cast<stk::StkFloat>(frequency));

        voice.bowed.controlChange(2, static_cast<stk::StkFloat>(bowPressure * 128.0f));

        voice.bowed.controlChange(4, static_cast<stk::StkFloat>(friction * 128.0f));

        voice.bowed.controlChange(100, static_cast<stk::StkFloat>(bowSpeed * 128.0f));

        const stk::StkFloat amplitude =
            static_cast<stk::StkFloat>(std::clamp(0.05f + strength * 0.95f, 0.05f, 1.0f));

        const stk::StkFloat attackRate = static_cast<stk::StkFloat>(
            std::max(0.0001, 0.005 / std::max(0.01, static_cast<double>(attackMs) * 0.001)));

        voice.bowed.startBowing(amplitude, attackRate);

        voice.envelope.reset();
        voice.envelope.setAttackMs(attackMs);
        voice.envelope.setReleaseMs(releaseMs);
        voice.envelope.trigger(strength);

        voice.frequencyHz = frequency;
        voice.midiNote =
            static_cast<int>(std::lround(69.0f + 12.0f * std::log2(frequency / 440.0f)));
        voice.strength = strength;
        voice.active = true;
        voice.releasing = false;
    }

    double sampleRate = 44100.0;

    float bowPressure = 0.5f;
    float bowSpeed = 0.5f;
    float friction = 0.127f;

    float attackMs = 50.0f;
    float releaseMs = 200.0f;

    std::array<Voice, kMaxVoices> voices;
};

#pragma once

#include <electrobow_dsp.h>

#include <array>
#include <cassert>
#include <cmath>
#include <memory>
#include <new>
#include <stdexcept>

// JUCE-facing ownership adapter. All filtering, state and audio storage live
// in rust/dsp; this class contains no DSP implementation. Use on one audio
// thread, with prepare/destruction only while processing is stopped.
class FilterBank {
  public:
    static constexpr int numBands = 4;
    static constexpr int maxChannels = 2;
    static constexpr std::array<double, 3> nominalCrossoversHz{200.0, 800.0, 3000.0};

    void prepare(double sampleRate, int maximumBlockSize, int channels) {
        if (!std::isfinite(sampleRate) || sampleRate <= 0.0 || maximumBlockSize < 1 ||
            channels < 1 || channels > maxChannels)
            throw std::invalid_argument("Invalid FilterBank preparation settings");

        // Build a replacement first; failed preparation leaves the old bank
        // intact. The unique_ptr ensures exactly one Rust-side destruction.
        Handle replacement(eb_filter_bank_create(sampleRate, maximumBlockSize, channels));
        if (!replacement)
            throw std::bad_alloc();
        eb_filter_bank_crossovers(replacement.get(), crossoversHz.data());
        handle = std::move(replacement);
        capacity = maximumBlockSize;
        channelCount = channels;
    }

    void processAttack(double milliseconds) noexcept {
        eb_filter_bank_attack(handle.get(), milliseconds);
    }
    void processCharacter(double amount) noexcept {
        eb_filter_bank_character(handle.get(), amount);
    }
    void process(const float* const* input, float* const* output, int samples,
                 EbBowControls controls) noexcept {
        const auto ok = eb_bow_process(handle.get(), input, output, samples, controls);
        assert(ok != 0);
        (void)ok;
    }
    void release() noexcept {
        handle.reset();
        capacity = 0;
        channelCount = 0;
    }
    void reset() noexcept {
        eb_filter_bank_reset(handle.get());
    }
    float getInputLevel() const noexcept {
        return eb_bow_input_level(handle.get());
    }
    int getCapacity() const noexcept {
        return capacity;
    }
    int getNumChannels() const noexcept {
        return channelCount;
    }
    int getNumSamples() const noexcept {
        return eb_filter_bank_samples(handle.get());
    }
    const std::array<double, 3>& getCrossoversHz() const noexcept {
        return crossoversHz;
    }

    // Split at most getCapacity() samples, optionally modify getBandData(),
    // then recombine. Access borrowed pointers only between calls and reacquire
    // them after split/reset. Preparation/destruction frees their storage.
    void split(const float* const* input, int samples) noexcept {
        const auto ok = eb_filter_bank_split(handle.get(), input, samples);
        assert(ok != 0);
        (void)ok;
    }
    float* getBandData(int band, int channel) noexcept {
        auto* result = eb_filter_bank_band(handle.get(), band, channel);
        assert(result != nullptr);
        return result;
    }
    const float* getBandData(int band, int channel) const noexcept {
        auto* result = eb_filter_bank_band(handle.get(), band, channel);
        assert(result != nullptr);
        return result;
    }
    void recombine(float* const* output) const noexcept {
        const auto ok = eb_filter_bank_recombine(handle.get(), output);
        assert(ok != 0);
        (void)ok;
    }

    FilterBank() = default;
    FilterBank(const FilterBank&) = delete;
    FilterBank& operator=(const FilterBank&) = delete;
    FilterBank(FilterBank&&) = delete;
    FilterBank& operator=(FilterBank&&) = delete;

  private:
    struct Deleter {
        void operator()(EbFilterBank* bank) const noexcept {
            eb_filter_bank_destroy(bank);
        }
    };
    using Handle = std::unique_ptr<EbFilterBank, Deleter>;
    Handle handle;
    int capacity = 0;
    int channelCount = 0;
    std::array<double, 3> crossoversHz{};
};

#pragma once

#include <algorithm>
#include <array>
#include <cassert>
#include <cmath>
#include <stdexcept>
#include <vector>

// Four parallel, amplitude-complementary bands. With Lf a first-order
// bilinear low-pass at f Hz, the transfers are:
//   L200, L800 - L200, L3000 - L800, 1 - L3000.
// Their sum is exactly 1 (not an all-pass). Individual bands have phase shift
// and gentle 6 dB/octave skirts; the unprocessed sum has neither delay nor
// phase rotation. Float band storage introduces only rounding error.
class FilterBank {
  public:
    static constexpr int numBands = 4;
    static constexpr int maxChannels = 2;
    static constexpr std::array<double, 3> nominalCrossoversHz{200.0, 800.0, 3000.0};

    // Call only off the audio thread. Preparation clears all history.
    void prepare(double sampleRate, int maximumBlockSize, int channels) {
        if (!std::isfinite(sampleRate) || sampleRate <= 0.0 || maximumBlockSize < 1 ||
            channels < 1 || channels > maxChannels)
            throw std::invalid_argument("Invalid FilterBank preparation settings");

        capacity = maximumBlockSize;
        channelCount = channels;
        // Preserve ordering and frequency ratios even at unusually low rates.
        const double scale = std::min(1.0, 0.45 * sampleRate / nominalCrossoversHz.back());
        constexpr double pi = 3.14159265358979323846;
        for (std::size_t i = 0; i < coefficients.size(); ++i) {
            crossoversHz[i] = nominalCrossoversHz[i] * scale;
            const double g = std::tan(pi * crossoversHz[i] / sampleRate);
            coefficients[i] = g / (1.0 + g);
        }
        for (auto& band : bands)
            for (auto& channel : band)
                channel.assign(static_cast<std::size_t>(capacity), 0.0f);
        reset();
    }

    void reset() noexcept {
        states = {};
        validSamples = 0;
        for (auto& band : bands)
            for (auto& channel : band)
                std::fill(channel.begin(), channel.end(), 0.0f);
    }

    int getCapacity() const noexcept {
        return capacity;
    }
    int getNumChannels() const noexcept {
        return channelCount;
    }
    int getNumSamples() const noexcept {
        return validSamples;
    }
    const std::array<double, 3>& getCrossoversHz() const noexcept {
        return crossoversHz;
    }

    // Audio-thread-only API: split a chunk of at most getCapacity() samples,
    // optionally process each band in place, then recombine. Buffers remain
    // valid until prepare(), but their contents are replaced by the next split.
    // Input/output may alias because all input is consumed before recombination.
    void split(const float* const* input, int samples) noexcept {
        assert(capacity > 0 && samples >= 0 && samples <= capacity);
        validSamples = samples;
        for (int channel = 0; channel < channelCount; ++channel) {
            auto& state = states[static_cast<std::size_t>(channel)];
            std::array<float*, numBands> output{};
            for (int band = 0; band < numBands; ++band)
                output[static_cast<std::size_t>(band)] = getBandData(band, channel);

            for (int sample = 0; sample < samples; ++sample) {
                const double x = input[channel][sample];
                std::array<double, 3> low{};
                for (std::size_t i = 0; i < low.size(); ++i) {
                    // Trapezoidal integrator: one independent state per
                    // crossover per channel, with double internal precision.
                    const double v = (x - state[i]) * coefficients[i];
                    low[i] = v + state[i];
                    state[i] = low[i] + v;
                }
                output[0][sample] = static_cast<float>(low[0]);
                output[1][sample] = static_cast<float>(low[1] - low[0]);
                output[2][sample] = static_cast<float>(low[2] - low[1]);
                output[3][sample] = static_cast<float>(x - low[2]);
            }
            // Prevent denormal tails, including standalone use without JUCE's
            // ScopedNoDenormals. This floor is below -600 dBFS.
            for (auto& value : state)
                if (std::abs(value) < 1.0e-30)
                    value = 0.0;
        }
    }

    float* getBandData(int band, int channel) noexcept {
        assert(band >= 0 && band < numBands && channel >= 0 && channel < channelCount);
        return bands[static_cast<std::size_t>(band)][static_cast<std::size_t>(channel)].data();
    }

    const float* getBandData(int band, int channel) const noexcept {
        assert(band >= 0 && band < numBands && channel >= 0 && channel < channelCount);
        return bands[static_cast<std::size_t>(band)][static_cast<std::size_t>(channel)].data();
    }

    void recombine(float* const* output) const noexcept {
        for (int channel = 0; channel < channelCount; ++channel)
            for (int sample = 0; sample < validSamples; ++sample) {
                double sum = 0.0;
                for (int band = 0; band < numBands; ++band)
                    sum += getBandData(band, channel)[sample];
                output[channel][sample] = static_cast<float>(sum);
            }
    }

  private:
    int capacity = 0;
    int channelCount = 0;
    int validSamples = 0;
    std::array<double, 3> coefficients{};
    std::array<double, 3> crossoversHz{};
    std::array<std::array<double, 3>, maxChannels> states{};
    std::array<std::array<std::vector<float>, maxChannels>, numBands> bands;
};

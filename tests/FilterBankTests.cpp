#include "../Source/FilterBank.h"
#include "../Source/PluginProcessor.h"
#include "reference/CppFilterBankReference.h"

#include <algorithm>
#include <vector>

#include <chrono>
#include <complex>
#include <cstdlib>
#include <iomanip>
#include <iostream>
#include <limits>
#include <random>
#include <string>

namespace {
constexpr double pi = 3.14159265358979323846;
using Signal = std::array<std::vector<float>, 2>;
using Bands = std::array<Signal, FilterBank::numBands>;

void require(bool condition, const std::string& message) {
    if (!condition) {
        std::cerr << "FAIL: " << message << '\n';
        std::exit(1);
    }
}

Signal makeSignal(int length) {
    Signal result;
    std::mt19937 random(12345);
    std::uniform_real_distribution<float> noise(-0.9f, 0.9f);
    for (auto& channel : result) {
        channel.resize(static_cast<std::size_t>(length));
        for (auto& sample : channel)
            sample = noise(random);
    }
    return result;
}

// Retain every band as well as the sum, so a dry bypass cannot pass these tests.
Signal render(FilterBank& bank, const Signal& input, const std::vector<int>& blockSizes,
              Bands* captured = nullptr) {
    Signal output{std::vector<float>(input[0].size()), std::vector<float>(input[1].size())};
    if (captured != nullptr)
        for (auto& band : *captured)
            for (auto& channel : band)
                channel.resize(input[0].size());
    int offset = 0;
    std::size_t block = 0;
    while (offset < static_cast<int>(input[0].size())) {
        const int count = std::min(blockSizes[block++ % blockSizes.size()],
                                   static_cast<int>(input[0].size()) - offset);
        const float* in[]{input[0].data() + offset, input[1].data() + offset};
        float* out[]{output[0].data() + offset, output[1].data() + offset};
        bank.split(in, count);
        require(bank.getNumSamples() == count, "valid band sample count");
        if (captured != nullptr)
            for (int band = 0; band < FilterBank::numBands; ++band)
                for (int channel = 0; channel < bank.getNumChannels(); ++channel)
                    std::copy_n(bank.getBandData(band, channel), count,
                                (*captured)[static_cast<std::size_t>(band)]
                                           [static_cast<std::size_t>(channel)]
                                               .data() +
                                    offset);
        bank.recombine(out);
        offset += count;
    }
    return output;
}

std::complex<double> response(const std::vector<float>& impulse, double frequency, double rate) {
    const auto rotation = std::polar(1.0, -2.0 * pi * frequency / rate);
    std::complex<double> phase{1.0, 0.0}, sum{};
    for (float sample : impulse) {
        sum += static_cast<double>(sample) * phase;
        phase *= rotation;
    }
    return sum;
}

std::array<std::complex<double>, 4> expectedResponse(double frequency, double rate,
                                                     const std::array<double, 3>& cutoffs) {
    const auto z = std::polar(1.0, -2.0 * pi * frequency / rate);
    std::array<std::complex<double>, 3> low{};
    for (std::size_t i = 0; i < low.size(); ++i) {
        const double g = std::tan(pi * cutoffs[i] / rate);
        const double b = g / (1.0 + g);
        const double a = (g - 1.0) / (g + 1.0);
        low[i] = b * (1.0 + z) / (1.0 + a * z);
    }
    return {low[0], low[1] - low[0], low[2] - low[1], 1.0 - low[2]};
}

void measureReconstruction(double rate) {
    FilterBank bank;
    bank.prepare(rate, 2048, 2);
    Signal impulse{std::vector<float>(32768), std::vector<float>(32768)};
    impulse[0][0] = 1.0f;
    impulse[1][0] = -0.7f;
    Bands bands;
    const auto output = render(bank, impulse, {1, 17, 64, 257, 2048}, &bands);
    double impulseError = 0.0;
    for (std::size_t sample = 0; sample < impulse[0].size(); ++sample)
        impulseError = std::max(
            impulseError, std::abs(static_cast<double>(output[0][sample]) - impulse[0][sample]));
    require(impulseError < 2.0e-7, "undelayed impulse and tail cancellation");
    require(std::max_element(output[0].begin(), output[0].end()) == output[0].begin(),
            "zero-sample impulse latency");

    std::vector<double> frequencies{0.0, 200.0, 800.0, 3000.0, rate * 0.5};
    for (int i = 0; i <= 128; ++i)
        frequencies.push_back(10.0 * std::pow(rate * 0.499 / 10.0, i / 128.0));
    double maxGainDb = 0.0, maxPhaseDegrees = 0.0, maxBandError = 0.0;
    for (double frequency : frequencies) {
        if (frequency > rate * 0.5)
            continue;
        const auto sum = response(output[0], frequency, rate);
        maxGainDb = std::max(maxGainDb, std::abs(20.0 * std::log10(std::abs(sum))));
        maxPhaseDegrees = std::max(maxPhaseDegrees, std::abs(std::arg(sum) * 180.0 / pi));
        const auto expected = expectedResponse(frequency, rate, bank.getCrossoversHz());
        for (std::size_t band = 0; band < bands.size(); ++band)
            maxBandError = std::max(
                maxBandError, std::abs(response(bands[band][0], frequency, rate) - expected[band]));
    }
    require(maxGainDb < 0.00001, "summed frequency response flatness including DC/Nyquist");
    require(maxPhaseDegrees < 0.0001, "summed phase response");
    require(maxBandError < 2.0e-6, "individual complex band responses");

    // Each band must dominate in its intended range (with broad overlap).
    if (rate >= 44100.0) {
        const std::array<double, 4> centres{50.0, 400.0, 1600.0, 10000.0};
        for (std::size_t band = 0; band < bands.size(); ++band) {
            const double intended = std::abs(response(bands[band][0], centres[band], rate));
            for (std::size_t other = 0; other < bands.size(); ++other)
                if (other != band)
                    require(intended > std::abs(response(bands[other][0], centres[band], rate)),
                            "frequency-selective independent bands");
        }
    }

    bank.reset();
    const auto noise = makeSignal(65536);
    const auto reconstructed = render(bank, noise, {1, 16, 127, 512, 2048});
    double peakError = 0.0, errorEnergy = 0.0, inputEnergy = 0.0, outputEnergy = 0.0;
    for (std::size_t channel = 0; channel < noise.size(); ++channel)
        for (std::size_t sample = 0; sample < noise[channel].size(); ++sample) {
            const double x = noise[channel][sample], y = reconstructed[channel][sample];
            require(std::isfinite(y), "finite noise output");
            peakError = std::max(peakError, std::abs(y - x));
            errorEnergy += (y - x) * (y - x);
            inputEnergy += x * x;
            outputEnergy += y * y;
        }
    const double nullDb = 10.0 * std::log10(errorEnergy / inputEnergy);
    const double gainDb = 10.0 * std::log10(outputEnergy / inputEnergy);
    require(peakError < 2.0e-7 && nullDb < -135.0 && std::abs(gainDb) < 0.00001,
            "noise reconstruction and unity gain");
    std::cout << rate << " Hz: peak=" << peakError << ", null=" << nullDb << " dBr, gain=" << gainDb
              << " dB, response=" << maxGainDb << " dB, phase=" << maxPhaseDegrees
              << " deg, impulse=" << impulseError << ", band complex error=" << maxBandError
              << '\n';
}

void testStateAndBands() {
    const auto input = makeSignal(16384);
    FilterBank bank;
    bank.prepare(48000.0, 2048, 2);
    Bands referenceBands, chunkedBands;
    const auto reference = render(bank, input, {2048}, &referenceBands);
    bank.reset();
    const auto chunked = render(bank, input, {1, 3, 17, 256, 511, 2048}, &chunkedBands);
    require(reference == chunked && referenceBands == chunkedBands,
            "block partition invariance of all four bands");

    FilterBank mono;
    mono.prepare(48000.0, 2048, 1);
    Bands monoBands;
    render(mono, input, {2048}, &monoBands);
    for (std::size_t band = 0; band < monoBands.size(); ++band)
        require(monoBands[band][0] == referenceBands[band][0], "independent mono/stereo states");

    for (int silentChannel = 0; silentChannel < 2; ++silentChannel) {
        bank.reset();
        auto isolated = input;
        std::fill(isolated[static_cast<std::size_t>(silentChannel)].begin(),
                  isolated[static_cast<std::size_t>(silentChannel)].end(), 0.0f);
        Bands isolatedBands;
        render(bank, isolated, {2048}, &isolatedBands);
        for (const auto& band : isolatedBands)
            for (float sample : band[static_cast<std::size_t>(silentChannel)])
                require(sample == 0.0f, "zero crosstalk in individual bands");
    }

    // Reprepare after active processing must clear history and change coefficients.
    bank.prepare(96000.0, 2048, 2);
    FilterBank fresh;
    fresh.prepare(96000.0, 2048, 2);
    Bands freshBands, preparedBands;
    render(bank, input, {2048}, &preparedBands);
    render(fresh, input, {2048}, &freshBands);
    require(preparedBands == freshBands, "reprepare clears all channel histories");

    // Exercise DC/steps, alternating Nyquist samples, tiny signals and headroom.
    for (float level : {0.0f, 1.0e-20f, 0.5f, -1.0f, 4.0f}) {
        bank.reset();
        Signal transient{std::vector<float>(32768), std::vector<float>(32768)};
        for (std::size_t i = 101; i < 16384; ++i) {
            transient[0][i] = level;
            transient[1][i] = i % 2 == 0 ? level : -level;
        }
        const auto out = render(bank, transient, {2048});
        for (std::size_t channel = 0; channel < transient.size(); ++channel)
            for (std::size_t i = 0; i < transient[channel].size(); ++i)
                require(std::abs(out[channel][i] - transient[channel][i]) <=
                            std::max(1.0e-27f, std::abs(level) * 2.0e-7f),
                        "DC, step, Nyquist, low-level and headroom reconstruction");
    }

    // Per-band buffers are writable and actually feed the output.
    bank.reset();
    const float* in[]{input[0].data(), input[1].data()};
    bank.split(in, 2048);
    std::array<float, 2048> expected{};
    std::copy_n(bank.getBandData(2, 0), 2048, expected.data());
    for (int band = 0; band < FilterBank::numBands; ++band)
        if (band != 2)
            std::fill_n(bank.getBandData(band, 0), 2048, 0.0f);
    Signal output{std::vector<float>(2048), std::vector<float>(2048)};
    float* out[]{output[0].data(), output[1].data()};
    bank.recombine(out);
    require(std::equal(expected.begin(), expected.end(), output[0].begin()),
            "independent band access feeds recombination");
    bank.split(in, 0);
    require(bank.getNumSamples() == 0, "zero-length split");
    bank.reset();
    for (int band = 0; band < FilterBank::numBands; ++band)
        for (int channel = 0; channel < 2; ++channel)
            for (int sample = 0; sample < bank.getCapacity(); ++sample)
                require(bank.getBandData(band, channel)[sample] == 0.0f,
                        "reset clears band buffers");

    bank.prepare(4000.0, 64, 1);
    require(bank.getCrossoversHz().back() <= 1800.0, "low sample-rate Nyquist guard");
    for (double invalid : {0.0, -1.0, std::numeric_limits<double>::infinity(),
                           std::numeric_limits<double>::quiet_NaN()}) {
        bool threw = false;
        try {
            bank.prepare(invalid, 64, 1);
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        require(threw, "reject invalid sample rates off the audio thread");
    }
    std::cout
        << "PASS: band access, channel isolation, partitioning, reset, reprepare, transients\n";
}

// Compare every independent band with the committed C++ implementation, not
// just the sum: a bypass or a changed crossover cannot hide behind cancellation.
void testCppParity() {
    double maximumBandError = 0.0, maximumOutputError = 0.0;
    const auto signal = makeSignal(16384);
    for (double rate :
         {4000.0, 8000.0, 22050.0, 44100.0, 48000.0, 88200.0, 96000.0, 192000.0, 384000.0}) {
        for (int channels : {1, 2}) {
            FilterBank rust;
            CppFilterBankReference cpp;
            rust.prepare(rate, 2048, channels);
            cpp.prepare(rate, 2048, channels);
            for (int pass = 0; pass < 2; ++pass) {
                if (pass != 0) {
                    rust.reset();
                    cpp.reset();
                }
                int offset = 0;
                for (int block = 0; offset < static_cast<int>(signal[0].size()); ++block) {
                    const std::array<int, 5> sizes{1, 17, 64, 257, 2048};
                    const int count =
                        std::min(sizes[static_cast<std::size_t>(block) % sizes.size()],
                                 static_cast<int>(signal[0].size()) - offset);
                    const float* input[]{signal[0].data() + offset, signal[1].data() + offset};
                    rust.split(input, count);
                    cpp.split(input, count);
                    for (int band = 0; band < FilterBank::numBands; ++band)
                        for (int channel = 0; channel < channels; ++channel)
                            for (int i = 0; i < count; ++i)
                                maximumBandError =
                                    std::max(maximumBandError,
                                             std::abs(static_cast<double>(
                                                          rust.getBandData(band, channel)[i]) -
                                                      cpp.getBandData(band, channel)[i]));
                    std::array<std::array<float, 2048>, 2> rustOutput{}, cppOutput{};
                    float* rustPointers[]{rustOutput[0].data(), rustOutput[1].data()};
                    float* cppPointers[]{cppOutput[0].data(), cppOutput[1].data()};
                    rust.recombine(rustPointers);
                    cpp.recombine(cppPointers);
                    for (int channel = 0; channel < channels; ++channel)
                        for (int i = 0; i < count; ++i)
                            maximumOutputError =
                                std::max(maximumOutputError,
                                         std::abs(static_cast<double>(rustPointers[channel][i]) -
                                                  cppPointers[channel][i]));
                    offset += count;
                }
            }
        }
    }
    require(maximumBandError < 2.0e-7 && maximumOutputError < 2.0e-7,
            "Rust preserves the original C++ bands and reconstruction");
    std::cout << "C++/Rust parity: max band error=" << maximumBandError
              << ", max output error=" << maximumOutputError << '\n';
}

void testProcessor() {
    ElectroBowAudioProcessor processor;
    processor.setUiParameter("attackMs", 0.0f);
    juce::MidiBuffer midi;
    for (int channels : {1, 2}) {
        auto layout = processor.getBusesLayout();
        layout.inputBuses.set(0, juce::AudioChannelSet::canonicalChannelSet(channels));
        layout.outputBuses.set(0, juce::AudioChannelSet::canonicalChannelSet(channels));
        require(processor.setBusesLayout(layout), "mono/stereo bus layout");
        for (double rate : {8000.0, 44100.0, 48000.0, 96000.0, 192000.0}) {
            processor.prepareToPlay(rate, 64);
            require(processor.getLatencySamples() == 0 && processor.getTailLengthSeconds() == 0.0,
                    "zero reported latency and tail");
            for (int size : {0, 1, 17, 64, 127, 512, 4097}) {
                const auto original = makeSignal(size);
                juce::AudioBuffer<float> buffer(channels, size);
                for (int channel = 0; channel < channels; ++channel)
                    buffer.copyFrom(channel, 0, original[static_cast<std::size_t>(channel)].data(),
                                    size);
                processor.processBlock(buffer, midi);
                for (int channel = 0; channel < channels; ++channel)
                    for (int sample = 0; sample < size; ++sample)
                        require(std::abs(buffer.getSample(channel, sample) -
                                         original[static_cast<std::size_t>(channel)]
                                                 [static_cast<std::size_t>(sample)]) < 2.0e-7f,
                                "processor reconstruction including oversized blocks");
            }
            require(processor.getActiveVoiceCount() == 0, "no bow processing in Phase 1");
            processor.releaseResources();
        }
    }
    auto layout = processor.getBusesLayout();
    layout.inputBuses.set(0, juce::AudioChannelSet::mono());
    layout.outputBuses.set(0, juce::AudioChannelSet::stereo());
    require(!processor.isBusesLayoutSupported(layout), "reject mismatched bus layout");
    layout.inputBuses.set(0, juce::AudioChannelSet::create5point1());
    layout.outputBuses.set(0, juce::AudioChannelSet::create5point1());
    require(!processor.isBusesLayoutSupported(layout), "reject unsupported surround layout");
    processor.prepareToPlay(48000.0, 0);
    juce::AudioBuffer<float> buffer(2, 7);
    buffer.clear();
    processor.processBlock(buffer, midi);
    std::cout
        << "PASS: processor mono/stereo, variable/oversized/empty blocks, latency and lifecycle\n";
}

void testAttackIntegration() {
    for (double rate : {8000.0, 44100.0, 48000.0, 96000.0, 192000.0}) {
        ElectroBowAudioProcessor dry, wet;
        dry.setUiParameter("attackMs", 0.0f);
        wet.setUiParameter("attackMs", 500.0f);
        dry.prepareToPlay(rate, 64);
        wet.prepareToPlay(rate, 64);
        juce::MidiBuffer midi;
        double difference = 0.0;
        for (int block = 0; block < 20; ++block) {
            juce::AudioBuffer<float> a(2, 257), b(2, 257);
            for (int i = 0; i < 257; ++i) {
                const float x = static_cast<float>(
                    0.5 * std::sin(2.0 * pi * rate * 0.2 * (block * 257 + i) / rate));
                a.setSample(0, i, x);
                a.setSample(1, i, x * 0.5f);
            }
            b.makeCopyOf(a);
            dry.processBlock(a, midi);
            wet.processBlock(b, midi);
            for (int i = 0; i < 257; ++i) {
                require(std::isfinite(b.getSample(0, i)), "finite attack output");
                require(std::abs(b.getSample(0, i) * 0.5f - b.getSample(1, i)) < 1e-7f,
                        "attack stereo linking preserves channel ratio");
                difference += std::abs(a.getSample(0, i) - b.getSample(0, i));
            }
        }
        require(difference > 1.0, "Attack parameter reaches Rust DSP");
        require(wet.getLatencySamples() == 0, "attack adds no buffering latency");
    }
}

void testCharacterIntegration() {
    ElectroBowAudioProcessor source, restored;
    source.setUiParameter("bowCharacter", 0.75f);
    juce::MemoryBlock state;
    source.getStateInformation(state);
    restored.setStateInformation(state.getData(), static_cast<int>(state.getSize()));
    require(std::abs(static_cast<float>(restored.getUiState()
                                            .getProperty("parameters", {})
                                            .getProperty("bowCharacter", 0.0f)) -
                     0.75f) < 1e-6f,
            "character state roundtrip");
    juce::MemoryBlock legacy;
    juce::MemoryOutputStream stream(legacy, false);
    stream.writeInt(0x45424F57);
    stream.writeInt(1);
    for (float value : {0.5f, 0.5f, 0.127f, 50.0f, 200.0f})
        stream.writeFloat(value);
    restored.setStateInformation(legacy.getData(), static_cast<int>(legacy.getSize()));
    require(static_cast<float>(restored.getUiState()
                                   .getProperty("parameters", {})
                                   .getProperty("bowCharacter", -1.0f)) == 0.0f,
            "legacy sessions use smooth character");
    source.prepareToPlay(48000.0, 64);
    restored.prepareToPlay(48000.0, 64);
    juce::MidiBuffer midi;
    double difference = 0.0;
    for (int block = 0; block < 40; ++block) {
        juce::AudioBuffer<float> a(2, 257), b(2, 257);
        for (int i = 0; i < 257; ++i) {
            const float x = 0.3f * std::sin(static_cast<float>(block * 257 + i) * 0.14f);
            a.setSample(0, i, x);
            a.setSample(1, i, x);
        }
        b.makeCopyOf(a);
        source.processBlock(a, midi);
        restored.processBlock(b, midi);
        for (int i = 0; i < 257; ++i) {
            require(std::isfinite(a.getSample(0, i)), "finite character output");
            require(a.getSample(0, i) == a.getSample(1, i), "linked character stereo");
            difference += std::abs(a.getSample(0, i) - b.getSample(0, i));
        }
    }
    require(difference > 1.0, "Bow Character reaches the audio path");
}

void testInvalidStateKeepsProcessing() {
    ElectroBowAudioProcessor processor, reference;
    for (auto* instance : {&processor, &reference}) {
        instance->setUiParameter("attackMs", 350.0f);
        instance->setUiParameter("bowCharacter", 0.65f);
        instance->prepareToPlay(48000.0, 64);
    }
    juce::MemoryBlock original;
    processor.getStateInformation(original);
    for (int version : {1, 2}) {
        for (int field = 0; field < (version == 1 ? 5 : 6); ++field) {
            for (float invalid :
                 {std::numeric_limits<float>::quiet_NaN(), std::numeric_limits<float>::infinity(),
                  -std::numeric_limits<float>::infinity()}) {
                juce::MemoryBlock state;
                juce::MemoryOutputStream stream(state, false);
                stream.writeInt(0x45424F57);
                stream.writeInt(version);
                for (int i = 0; i < (version == 1 ? 5 : 6); ++i)
                    stream.writeFloat(i == field ? invalid : 0.0f);
                processor.setStateInformation(state.getData(), static_cast<int>(state.getSize()));
                juce::MemoryBlock after;
                processor.getStateInformation(after);
                require(after == original, "non-finite state rejected without partial updates");
            }
        }
    }
    for (std::size_t length = 0; length < original.getSize(); ++length) {
        processor.setStateInformation(original.getData(), static_cast<int>(length));
        juce::MemoryBlock after;
        processor.getStateInformation(after);
        require(after == original, "truncated state rejected without partial updates");
    }
    juce::MidiBuffer midi;
    const auto signal = makeSignal(257);
    juce::AudioBuffer<float> actual(2, 257), expected(2, 257);
    for (int channel = 0; channel < 2; ++channel)
        actual.copyFrom(channel, 0, signal[static_cast<std::size_t>(channel)].data(), 257);
    expected.makeCopyOf(actual);
    processor.processBlock(actual, midi);
    reference.processBlock(expected, midi);
    for (int channel = 0; channel < 2; ++channel)
        for (int i = 0; i < 257; ++i)
            require(actual.getSample(channel, i) == expected.getSample(channel, i),
                    "invalid state must not bypass or freeze DSP");
}

void benchmark() {
    // Median of seven batches, each one second of stereo audio. This measures
    // DSP + processor dispatch, excluding host, GUI and audio-device scheduling.
    volatile double checksum = 0.0;
    for (int blockSize : {16, 64, 256, 1024}) {
        ElectroBowAudioProcessor processor;
        processor.setUiParameter("attackMs", 500.0f);
        processor.setUiParameter("bowCharacter", 1.0f);
        processor.prepareToPlay(48000.0, blockSize);
        juce::AudioBuffer<float> buffer(2, blockSize);
        const auto input = makeSignal(blockSize);
        const auto refill = [&] {
            for (int channel = 0; channel < 2; ++channel)
                buffer.copyFrom(channel, 0, input[static_cast<std::size_t>(channel)].data(),
                                blockSize);
        };
        juce::MidiBuffer midi;
        const int blocks = (48000 + blockSize - 1) / blockSize;
        for (int i = 0; i < 100; ++i) {
            refill();
            processor.processBlock(buffer, midi);
        }
        std::array<double, 7> timings{};
        for (auto& elapsed : timings) {
            const auto start = std::chrono::steady_clock::now();
            for (int i = 0; i < blocks; ++i) {
                // A host supplies new input, never the previous wet output.
                // Include this small buffer-copy overhead in reported CPU.
                refill();
                processor.processBlock(buffer, midi);
                checksum += buffer.getSample(0, i % blockSize);
            }
            elapsed =
                std::chrono::duration<double>(std::chrono::steady_clock::now() - start).count();
        }
        std::sort(timings.begin(), timings.end());
        const double frames = static_cast<double>(blocks * blockSize);
        const double cpu = timings[3] / (frames / 48000.0) * 100.0;
        std::cout << "CPU stereo 48 kHz / " << blockSize << ": " << cpu << "% of one core, "
                  << timings[3] / frames * 1.0e9 << " ns/frame\n";
    }
    require(std::isfinite(checksum), "benchmark finite output");
}
} // namespace

int main() {
    std::cout << std::setprecision(9);
    for (double rate :
         {4000.0, 8000.0, 22050.0, 44100.0, 48000.0, 88200.0, 96000.0, 192000.0, 384000.0})
        measureReconstruction(rate);
    testCppParity();
    testStateAndBands();
    testProcessor();
    testAttackIntegration();
    testCharacterIntegration();
    testInvalidStateKeepsProcessing();
    benchmark();
    std::cout << "All FilterBank tests passed.\n";
}

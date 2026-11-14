#include "../Source/FilterBank.h"
#include "../Source/PluginProcessor.h"

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

void testProcessor() {
    ElectroBowAudioProcessor processor;
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

void benchmark() {
    // Median of seven batches, each one second of stereo audio. This measures
    // DSP + processor dispatch, excluding host, GUI and audio-device scheduling.
    volatile double checksum = 0.0;
    for (int blockSize : {16, 64, 256, 1024}) {
        ElectroBowAudioProcessor processor;
        processor.prepareToPlay(48000.0, blockSize);
        juce::AudioBuffer<float> buffer(2, blockSize);
        const auto input = makeSignal(blockSize);
        for (int channel = 0; channel < 2; ++channel)
            buffer.copyFrom(channel, 0, input[static_cast<std::size_t>(channel)].data(), blockSize);
        juce::MidiBuffer midi;
        const int blocks = (48000 + blockSize - 1) / blockSize;
        for (int i = 0; i < 100; ++i)
            processor.processBlock(buffer, midi);
        std::array<double, 7> timings{};
        for (auto& elapsed : timings) {
            const auto start = std::chrono::steady_clock::now();
            for (int i = 0; i < blocks; ++i) {
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
    testStateAndBands();
    testProcessor();
    benchmark();
    std::cout << "All FilterBank tests passed.\n";
}

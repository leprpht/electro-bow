// Offline host for the actually packaged VST3. Stdout carries interleaved IEEE float samples.
#include <array>
#include <cstdio>
#include <iostream>
#include <juce_audio_processors_headless/juce_audio_processors_headless.h>
#include <stdexcept>
#include <vector>

namespace {
void require(bool ok, const char* reason) {
    if (!ok)
        throw std::runtime_error(reason);
}

int render(int argc, char** argv) {
    require(argc == 11 || argc == 13,
            "bundle rate channels block and six parameter values required");
    const auto bundle = juce::File::getCurrentWorkingDirectory().getChildFile(argv[1]);
    const auto rate = std::stod(argv[2]);
    const auto channels = std::stoi(argv[3]);
    const auto block = std::stoi(argv[4]);
    require((channels == 1 || channels == 2) && block > 0 && rate > 0, "invalid render format");
    juce::VST3PluginFormatHeadless format;
    juce::OwnedArray<juce::PluginDescription> types;
    format.findAllTypesForFile(types, bundle.getFullPathName());
    require(types.size() == 1 && types[0]->name == "ElectroBow",
            "expected packaged ElectroBow component");
    juce::String err;
    auto plugin = format.createInstanceFromDescription(*types[0], rate, block, err);
    if (!plugin)
        throw std::runtime_error(err.toStdString());
    auto layout = plugin->getBusesLayout();
    layout.getChannelSet(true, 0) = layout.getChannelSet(false, 0) =
        juce::AudioChannelSet::canonicalChannelSet(channels);
    require(plugin->setBusesLayout(layout), "rejected bus layout");
    const std::array<juce::String, 6> names{"Attack",     "Bow Character",     "Noise",
                                            "Saturation", "Spectral Movement", "Out"};
    std::array<bool, 6> seen{};
    for (auto* parameter : plugin->getParameters()) {
        for (std::size_t i = 0; i < names.size(); ++i) {
            if (parameter->getName(128) == names[i]) {
                require(!seen[i], "duplicate plugin parameter");
                parameter->setValueNotifyingHost(std::stof(argv[5 + i]) / (i == 0 ? 500.f : 1.f));
                seen[i] = true;
            }
        }
    }
    require(std::all_of(seen.begin(), seen.end(), [](bool b) { return b; }),
            "missing public parameter");
    if (argc == 13 && std::string(argv[11]) == "load") {
        juce::MemoryBlock state;
        require(juce::File(argv[12]).loadFileAsData(state), "cannot load state");
        plugin->setStateInformation(state.getData(), static_cast<int>(state.getSize()));
    }
    plugin->setRateAndBufferSizeDetails(rate, block);
    plugin->prepareToPlay(rate, block);
    require(plugin->getLatencySamples() == 0, "packaged latency");
    std::vector<float> data(static_cast<std::size_t>(channels * block));
    juce::AudioBuffer<float> buffer(channels, block);
    juce::MidiBuffer midi;
    while (const auto words = std::fread(data.data(), sizeof(float), data.size(), stdin)) {
        require(words % channels == 0, "partial input frame");
        const auto length = static_cast<int>(words / channels);
        for (int j = 0; j < channels; ++j)
            for (int i = 0; i < length; ++i)
                buffer.setSample(j, i, data[static_cast<std::size_t>(i * channels + j)]);
        juce::AudioBuffer<float> slice(buffer.getArrayOfWritePointers(), channels, length);
        plugin->processBlock(slice, midi);
        for (int i = 0; i < length; ++i)
            for (int j = 0; j < channels; ++j)
                data[static_cast<std::size_t>(i * channels + j)] = slice.getSample(j, i);
        require(std::fwrite(data.data(), sizeof(float), words, stdout) == words,
                "output write failed");
    }
    require(std::ferror(stdin) == 0, "input read failed");
    if (argc == 13 && std::string(argv[11]) == "save") {
        juce::MemoryBlock state;
        plugin->getStateInformation(state);
        require(juce::File(argv[12]).replaceWithData(state.getData(), state.getSize()),
                "cannot save state");
    }
    plugin->releaseResources();
    return 0;
}
} // namespace
int main(int argc, char** argv) {
    try {
        juce::ScopedJuceInitialiser_GUI init;
        return render(argc, argv);
    } catch (const std::exception& err) {
        std::cerr << err.what() << '\n';
        return 1;
    }
}

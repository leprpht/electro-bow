#include "PluginEditor.h"

#include <ElectroBowWebUIData.h>

#include <cstring>

namespace {
std::optional<juce::WebBrowserComponent::Resource> makeResource(const char* resourceName,
                                                                const char* mimeType) {
    int size = 0;
    const char* data = ElectroBowWebUIData::getNamedResource(resourceName, size);

    if (data == nullptr || size <= 0)
        return std::nullopt;

    std::vector<std::byte> bytes(static_cast<std::size_t>(size));
    std::memcpy(bytes.data(), data, static_cast<std::size_t>(size));

    return juce::WebBrowserComponent::Resource{std::move(bytes), mimeType};
}
} // namespace

ElectroBowAudioProcessorEditor::ElectroBowAudioProcessorEditor(ElectroBowAudioProcessor& p)
    : AudioProcessorEditor(&p), processorRef(p) {
    auto options =
        juce::WebBrowserComponent::Options{}
            .withBackend(juce::WebBrowserComponent::Options::Backend::webview2)
            .withNativeIntegrationEnabled()
            .withNativeFunction("getPluginState",
                                [this](const juce::Array<juce::var>&, auto complete) {
                                    complete(processorRef.getUiState());
                                })
            .withNativeFunction("setParameter",
                                [this](const juce::Array<juce::var>& args, auto complete) {
                                    if (args.size() >= 2)
                                        processorRef.setUiParameter(args[0].toString(),
                                                                    static_cast<float>(args[1]));

                                    complete(processorRef.getUiState());
                                })
            .withResourceProvider([this](const juce::String& url) { return getResource(url); });

    webComponent = std::make_unique<juce::WebBrowserComponent>(options);
    addAndMakeVisible(*webComponent);
    webComponent->goToURL(juce::WebBrowserComponent::getResourceProviderRoot());

    setSize(760, 540);
    startTimerHz(30);
}

ElectroBowAudioProcessorEditor::~ElectroBowAudioProcessorEditor() {
    stopTimer();
    webComponent.reset();
}

std::optional<juce::WebBrowserComponent::Resource>
ElectroBowAudioProcessorEditor::getResource(const juce::String& url) const {
    const auto path =
        url == "/"
            ? juce::String("index.html")
            : url.fromLastOccurrenceOf("/", false, false).upToFirstOccurrenceOf("?", false, false);

    if (path == "index.html")
        return makeResource("index_html", "text/html");

    if (path == "app.js")
        return makeResource("app_js", "text/javascript");

    if (path == "app.css")
        return makeResource("app_css", "text/css");

    return std::nullopt;
}

void ElectroBowAudioProcessorEditor::timerCallback() {
    if (webComponent != nullptr)
        webComponent->emitEventIfBrowserIsVisible("pluginState", processorRef.getUiState());
}

void ElectroBowAudioProcessorEditor::paint(juce::Graphics& g) {
    g.fillAll(juce::Colour(0xff090a0d));
}

void ElectroBowAudioProcessorEditor::resized() {
    if (webComponent != nullptr)
        webComponent->setBounds(getLocalBounds());
}

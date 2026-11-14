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
    setSize(760, 540);
    browserError.setJustificationType(juce::Justification::centred);
    browserError.setColour(juce::Label::textColourId, juce::Colours::white);
    addChildComponent(browserError);
    browserError.setBounds(getLocalBounds().reduced(30));

    auto options =
        juce::WebBrowserComponent::Options{}
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

#if JUCE_WINDOWS
    // The default folder is next to the DAW executable, often in Program Files.
    // Keep our browser profile in a writable, plugin-specific local directory.
    const auto browserData = juce::File::getSpecialLocation(juce::File::windowsLocalAppData)
                                 .getChildFile("ElectroBow")
                                 .getChildFile("WebView2");
    if (browserData.createDirectory().failed()) {
        browserError.setText("ElectroBow could not create its browser data folder:\n" +
                                 browserData.getFullPathName() +
                                 "\nCheck that your user account can write to this folder.",
                             juce::dontSendNotification);
        browserError.setVisible(true);
        return;
    }

    options =
        options.withBackend(juce::WebBrowserComponent::Options::Backend::webview2)
            .withWinWebView2Options(
                juce::WebBrowserComponent::Options::WinWebView2{}.withUserDataFolder(browserData));

    // JUCE otherwise silently falls back to IE, which cannot serve our embedded
    // resources or run the React UI. Give the user an actionable native message.
    if (!juce::WebBrowserComponent::areOptionsSupported(options)) {
        browserError.setText("ElectroBow could not start Microsoft Edge WebView2.\n"
                             "Install or repair the Microsoft Edge WebView2 Evergreen Runtime, "
                             "then restart your DAW.",
                             juce::dontSendNotification);
        browserError.setVisible(true);
        return;
    }
#endif

    webComponent = std::make_unique<juce::WebBrowserComponent>(options);
    addAndMakeVisible(*webComponent);
    webComponent->setBounds(getLocalBounds());
    webComponent->goToURL(juce::WebBrowserComponent::getResourceProviderRoot());

    startTimerHz(30);
}

ElectroBowAudioProcessorEditor::~ElectroBowAudioProcessorEditor() {
    stopTimer();
    webComponent.reset();
}

std::optional<juce::WebBrowserComponent::Resource>
ElectroBowAudioProcessorEditor::getResource(const juce::String& url) const {
    // Resource-provider URLs differ slightly between JUCE backends. Normalize
    // the final path component so query strings/fragments and leading slashes
    // cannot make an embedded asset look missing.
    auto path = url.fromLastOccurrenceOf("/", false, false);

    if (path.isEmpty())
        path = "index.html";

    path = path.upToFirstOccurrenceOf("?", false, false).upToFirstOccurrenceOf("#", false, false);

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
    browserError.setBounds(getLocalBounds().reduced(30));
    if (webComponent != nullptr)
        webComponent->setBounds(getLocalBounds());
}

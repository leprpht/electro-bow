#include "../Source/PluginEditor.h"

#include <iostream>

// Use the production editor, binary assets and JUCE implementation from the
// plugin target. A browser that falls back to IE must fail this test even if
// the C++ build itself succeeds.
class EditorSmokeTests final : public juce::JUCEApplication, private juce::Timer {
  public:
    const juce::String getApplicationName() override {
        return "ElectroBow editor smoke test";
    }
    const juce::String getApplicationVersion() override {
        return "1.0";
    }

    void initialise(const juce::String&) override {
        processor = std::make_unique<ElectroBowAudioProcessor>();
        openEditor();
        startTimer(100);
    }

    void shutdown() override {
        stopTimer();
        editor.reset();
        processor.reset();
    }

  private:
    void openEditor() {
        // Distinct from the frontend defaults, so a rendered but disconnected
        // React page cannot pass. Reopening also checks browser teardown/reuse.
        processor->setUiParameter("bowCharacter", completedLoads == 0 ? 0.73f : 0.29f);
        editor = std::make_unique<ElectroBowAudioProcessorEditor>(*processor);
        editor->addToDesktop(juce::ComponentPeer::windowIsTemporary);
        editor->setVisible(true);
        browser = nullptr;
        for (auto* child : editor->getChildren())
            if (auto* candidate = dynamic_cast<juce::WebBrowserComponent*>(child))
                browser = candidate;

        ready = false;
        evaluationPending = false;
        deadline = juce::Time::getMillisecondCounterHiRes() + 20000.0;
    }

    void finish(bool passed) {
        stopTimer();
        if (passed)
            std::cout << "Embedded HTML, React, CSS and native state loaded on open and reopen.\n";
        else
            std::cerr << "Editor failed to load embedded UI and native state: " << lastResult
                      << '\n';
        setApplicationReturnValue(passed ? 0 : 1);
        quit();
    }

    void timerCallback() override {
        if (ready) {
            if (++completedLoads == 2) {
                finish(true);
            } else {
                editor.reset();
                openEditor();
            }
            return;
        }

        if (browser == nullptr || juce::Time::getMillisecondCounterHiRes() > deadline) {
            finish(false);
            return;
        }

        if (evaluationPending)
            return;

        evaluationPending = true;
        const auto expectedValue = completedLoads == 0 ? "0.73" : "0.29";
        const auto script = juce::String(R"JS(
            (() => {
                const shell = document.querySelector('.app-shell');
                const pressure = document.querySelector('input[aria-label="Bow Character"]');
                const ready = !!shell && !!pressure
                    && document.querySelectorAll('input[type="range"]').length === 2
                    && !!document.querySelector('.status-live')
                    && parseFloat(getComputedStyle(shell).paddingTop) > 0
                    && Math.abs(Number(pressure.value) - )JS") +
                            expectedValue + R"JS() < 0.001;
                return {
                    ready,
                    url: location.href,
                    text: document.body ? document.body.innerText.slice(0, 200) : '',
                    sliders: document.querySelectorAll('input[type="range"]').length,
                    connected: !!document.querySelector('.status-live'),
                    padding: shell ? getComputedStyle(shell).paddingTop : '',
                    pressure: pressure ? pressure.value : ''
                };
            })()
        )JS";
        browser->evaluateJavascript(script, [this](auto result) {
            evaluationPending = false;
            if (const auto* value = result.getResult()) {
                ready = static_cast<bool>(value->getProperty("ready", false));
                lastResult = juce::JSON::toString(*value);
            } else if (const auto* error = result.getError()) {
                lastResult = error->message;
            }
        });
    }

    std::unique_ptr<ElectroBowAudioProcessor> processor;
    std::unique_ptr<ElectroBowAudioProcessorEditor> editor;
    juce::WebBrowserComponent* browser = nullptr;
    double deadline = 0.0;
    int completedLoads = 0;
    bool ready = false;
    bool evaluationPending = false;
    juce::String lastResult = "browser unavailable or JavaScript did not respond";
};

#if JUCE_WINDOWS
// Keep console output for CTest, but use the no-argument GUI entry point that
// JUCE compiled into the production plugin (not the _CONSOLE-only overload).
int main() {
    juce::JUCEApplicationBase::createInstance = []() -> juce::JUCEApplicationBase* {
        return new EditorSmokeTests();
    };
    return juce::JUCEApplicationBase::main();
}
#else
START_JUCE_APPLICATION(EditorSmokeTests)
#endif

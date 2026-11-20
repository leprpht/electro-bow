import type { Page } from "@playwright/test";
import { defaultState, type ParameterId, type PluginState } from "../src/types";

type TestBridge = {
  state: PluginState;
  writes: Array<{ id: ParameterId; value: number }>;
  emit: (state: PluginState) => void;
};

declare global {
  interface Window {
    __testBridge: TestBridge;
  }
}

export async function installNativeBridge(page: Page) {
  await page.addInitScript(
    (initialState) => {
      // Exercise the real JUCE JS interop, including native promise completions.
      const nativeWindow = window as unknown as {
        __JUCE__: {
          initialisationData: Record<string, string[]>;
          postMessage: (message: string) => void;
          backend: { emitByBackend: (name: string, payload: string) => void };
        };
      };
      const bridge: TestBridge = {
        state: initialState,
        writes: [],
        emit(state) {
          this.state = state;
          nativeWindow.__JUCE__.backend.emitByBackend("pluginState", JSON.stringify(state));
        },
      };
      window.__testBridge = bridge;
      nativeWindow.__JUCE__ = {
        initialisationData: {
          __juce__functions: ["getPluginState", "setParameter", "setGesture"],
          __juce__platform: [],
          __juce__registeredGlobalEventIds: [],
          __juce__sliders: [],
          __juce__toggles: [],
          __juce__comboBoxes: [],
        },
        postMessage(message) {
          const { eventId, payload } = JSON.parse(message);
          if (eventId !== "__juce__invoke") return;
          if (payload.name === "setParameter") {
            const [id, value] = payload.params as [ParameterId, number];
            bridge.writes.push({ id, value });
            bridge.state.parameters[id] = value;
          }
          queueMicrotask(() =>
            nativeWindow.__JUCE__.backend.emitByBackend(
              "__juce__complete",
              JSON.stringify({ promiseId: payload.resultId, result: bridge.state }),
            ),
          );
        },
      } as typeof nativeWindow.__JUCE__;
    },
    { ...defaultState, parameters: { ...defaultState.parameters, bowCharacter: 0.73 } },
  );
}

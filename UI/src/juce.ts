import { getNativeFunction } from "@juce-framework/webview";
import { defaultState, type PluginState, type ParameterId } from "./types";

export function hasNativeBridge(): boolean {
  const functions = window.__JUCE__?.initialisationData?.__juce__functions;
  return !!functions?.includes("getPluginState") && !!functions.includes("setParameter");
}

export async function getPluginState(): Promise<PluginState> {
  if (!hasNativeBridge()) return defaultState;
  return (await getNativeFunction("getPluginState")()) as PluginState;
}

export function updateParameter(id: ParameterId, value: number): Promise<unknown> {
  if (!hasNativeBridge()) return Promise.resolve();
  return getNativeFunction("setParameter")(id, value);
}

export function updateGesture(id: ParameterId, active: boolean): Promise<unknown> {
  if (!hasNativeBridge()) return Promise.resolve();
  return getNativeFunction("setGesture")(id, active);
}

export function subscribeToState(listener: (state: PluginState) => void): () => void {
  const backend = window.__JUCE__?.backend;

  if (!backend) return () => undefined;

  const handle = backend.addEventListener("pluginState", (payload) => {
    listener(payload as PluginState);
  });

  return () => backend.removeEventListener(handle);
}

import { getNativeFunction } from "@juce-framework/webview";
import type { PluginState, ParameterId } from "./types";

const getState = getNativeFunction("getPluginState");
const setParameter = getNativeFunction("setParameter");

export async function getPluginState(): Promise<PluginState> {
  return (await getState()) as PluginState;
}

export function updateParameter(id: ParameterId, value: number): Promise<unknown> {
  return setParameter(id, value);
}

export function subscribeToState(listener: (state: PluginState) => void): () => void {
  const backend = window.__JUCE__?.backend;

  if (!backend) return () => undefined;

  const handle = backend.addEventListener("pluginState", (payload) => {
    listener(payload as PluginState);
  });

  return () => backend.removeEventListener(handle);
}

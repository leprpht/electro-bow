import { useCallback, useEffect, useRef, useState } from "react";
import {
  getPluginState,
  hasNativeBridge,
  subscribeToState,
  updateGesture,
  updateParameter,
} from "../juce";
import { defaultState, type ParameterId, type PluginState } from "../types";

export function usePluginState() {
  const [state, setState] = useState<PluginState>(defaultState);
  const [connected, setConnected] = useState(false);
  const gestures = useRef(new Set<ParameterId>());

  useEffect(() => {
    if (!hasNativeBridge()) return;
    let mounted = true;
    let receivedEvent = false;

    function receive(nextState: PluginState) {
      if (!mounted) return;
      setState((current) => {
        const parameters = { ...nextState.parameters };
        for (const id of gestures.current) parameters[id] = current.parameters[id];
        return { ...nextState, parameters };
      });
      setConnected(true);
    }

    const unsubscribe = subscribeToState((nextState) => {
      receivedEvent = true;
      receive(nextState);
    });
    void getPluginState()
      .then((nextState) => {
        if (!receivedEvent) receive(nextState);
      })
      .catch(() => {
        if (mounted && !receivedEvent) setConnected(false);
      });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, []);

  const setParameter = useCallback((id: ParameterId, value: number) => {
    setState((current) => ({ ...current, parameters: { ...current.parameters, [id]: value } }));
    void updateParameter(id, value).catch(() => setConnected(false));
  }, []);

  const setGesture = useCallback((id: ParameterId, active: boolean) => {
    if (active) gestures.current.add(id);
    else gestures.current.delete(id);
    void updateGesture(id, active).catch(() => setConnected(false));
  }, []);

  return { state, connected, setParameter, setGesture };
}

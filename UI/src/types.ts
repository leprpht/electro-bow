export type ParameterId =
  "bowPressure" | "bowSpeed" | "friction" | "attackMs" | "naturalResonanceMs";

export type PluginState = {
  parameters: Record<ParameterId, number>;
  pitch: {
    note: string;
    frequencyHz: number;
    confidence: number;
  };
  voices: Array<{
    frequencyHz: number;
    strength: number;
  }>;
};

export const defaultState: PluginState = {
  parameters: {
    bowPressure: 0.5,
    bowSpeed: 0.5,
    friction: 0.127,
    attackMs: 50,
    naturalResonanceMs: 200,
  },
  pitch: {
    note: "—",
    frequencyHz: 0,
    confidence: 0,
  },
  voices: [],
};

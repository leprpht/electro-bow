export type ParameterId =
  | "bowCharacter"
  | "bowPressure"
  | "bowSpeed"
  | "friction"
  | "attackMs"
  | "naturalResonanceMs"
  | "sustain"
  | "noise"
  | "saturation"
  | "spectralMovement"
  | "out";

export type PluginState = {
  inputLevel: number;
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
  inputLevel: 0,
  parameters: {
    bowCharacter: 0.5,
    noise: 0.5,
    saturation: 0.5,
    spectralMovement: 0.5,
    out: 0.5,
    sustain: 0.5,
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

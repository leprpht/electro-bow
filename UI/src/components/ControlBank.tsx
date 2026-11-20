import * as styles from "../styles/panel.css";
import { Knob } from "./Knob";
import { defaultState, type ParameterId, type PluginState } from "../types";

type ControlBankProps = {
  side: "left" | "right";
  parameters: PluginState["parameters"];
  onChange: (id: ParameterId, value: number) => void;
  onGestureChange: (id: ParameterId, active: boolean) => void;
};

const banks = {
  left: [
    ["attackMs", "Attack", "Time for a picked note to swell toward its playing level.", 500],
    ["sustain", "Sustain", "Support the string as it decays.", 1],
    ["naturalResonanceMs", "Resonance", "Decay time of the instrument body reflections.", 1000],
  ],
  right: [
    [
      "bowPressure",
      "Bow Pressure",
      "Thicken the middle register and compress stronger strokes.",
      1,
    ],
    ["bowSpeed", "Bow Speed", "Move from a dark stroke to a brighter, more open tone.", 1],
    ["friction", "Bow Friction", "Bring out string harmonics in the low and middle register.", 1],
  ],
} as const;

export function ControlBank({ side, parameters, onChange, onGestureChange }: ControlBankProps) {
  return (
    <section
      className={`${styles.bank} control-bank control-bank-${side} flex flex-col justify-between`}
      aria-label={side === "left" ? "Envelope controls" : "Bow controls"}
    >
      {banks[side].map(([id, label, description, max]) => (
        <Knob
          key={id}
          label={label}
          description={description}
          value={parameters[id]}
          max={max}
          step={max > 1 ? 1 : 0.001}
          defaultValue={defaultState.parameters[id]}
          format={max > 1 ? (value) => `${Math.round(value)} ms` : undefined}
          onChange={(value) => onChange(id, value)}
          onGestureChange={(active) => onGestureChange(id, active)}
        />
      ))}
    </section>
  );
}

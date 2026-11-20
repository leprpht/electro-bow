import * as styles from "../styles/panel.css";
import { Knob } from "./Knob";
import type { ParameterId, PluginState } from "../types";

type ControlBankProps = {
  side: "left" | "right";
  parameters: PluginState["parameters"];
  onChange: (id: ParameterId, value: number) => void;
  onGestureChange: (id: ParameterId, active: boolean) => void;
};

const inactiveDescription = "This control is not active in the current audio engine.";

export function ControlBank({ side, parameters, onChange, onGestureChange }: ControlBankProps) {
  return (
    <section
      className={`${styles.bank} control-bank control-bank-${side} flex flex-col justify-between`}
      aria-label={side === "left" ? "Envelope controls" : "Bow controls"}
    >
      {side === "left" ? (
        <>
          <Knob
            label="Attack Reduction"
            description="Softens the pick transient."
            value={parameters.attackMs}
            max={500}
            step={1}
            defaultValue={50}
            format={(value) => `${Math.round(value / 5)}%`}
            onChange={(value) => onChange("attackMs", value)}
            onGestureChange={(active) => onGestureChange("attackMs", active)}
          />
          <Knob label="Sustain" description={inactiveDescription} value={0.5} disabled />
          <Knob
            label="Resonance"
            description={inactiveDescription}
            value={parameters.naturalResonanceMs}
            max={1000}
            disabled
          />
        </>
      ) : (
        <>
          <Knob
            label="Bow Pressure"
            description={inactiveDescription}
            value={parameters.bowPressure}
            disabled
          />
          <Knob
            label="Bow Speed"
            description={inactiveDescription}
            value={parameters.bowSpeed}
            disabled
          />
          <Knob
            label="Bow Friction"
            description={inactiveDescription}
            value={parameters.friction}
            disabled
          />
        </>
      )}
    </section>
  );
}

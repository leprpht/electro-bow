import { InstrumentPanel } from "./components/InstrumentPanel";
import { Knob, type DialPosition } from "./components/Knob";
import { defaultState, type ParameterId } from "./types";
import { usePluginState } from "./hooks/usePluginState";

type FaceplateDial = {
  id: ParameterId;
  label: string;
  description: string;
  position: DialPosition;
};

const dials: FaceplateDial[] = [
  {
    id: "attackMs",
    label: "Attack",
    description: "Set the time for a picked note to swell.",
    position: { x: 235, y: 323, diameter: 156 },
  },
  {
    id: "bowCharacter",
    label: "Bow Character",
    description:
      "Warm the middle register, soften the treble, and blend in the three bow textures.",
    position: { x: 649, y: 324, diameter: 156 },
  },
  {
    id: "out",
    label: "Out",
    description: "Set output level. Noon is unity gain.",
    position: { x: 1053, y: 323, diameter: 156 },
  },
  {
    id: "noise",
    label: "Noise",
    description: "Blend string-driven bow noise into the attack and sustain.",
    position: { x: 424, y: 582, diameter: 144 },
  },
  {
    id: "saturation",
    label: "Saturation",
    description: "Soften and thicken the low-mid harmonics.",
    position: { x: 652, y: 582, diameter: 144 },
  },
  {
    id: "spectralMovement",
    label: "Spectral Movement",
    description: "Vary the balance of the woody mids and the bright edge over time.",
    position: { x: 880, y: 582, diameter: 144 },
  },
];

// The faceplate gives short swells more travel while preserving the host's 0–500 ms range.
// 0, 50 and 500 ms fall at the start, middle and end of the dial.
const attackScale = {
  toPosition: (ms: number) => Math.log1p(ms / 6.25) / Math.log(81),
  fromPosition: (position: number) => 6.25 * (81 ** position - 1),
};
const milliseconds = (ms: number) => `${Math.round(ms)} ms`;
const decibels = (out: number) =>
  out === 0 ? "−∞ dB" : `${(20 * Math.log10(2 * out)).toFixed(1)} dB`;

function App() {
  const { state, connected, setParameter, setGesture } = usePluginState();
  return (
    <InstrumentPanel>
      {dials.map(({ id, ...dial }) => (
        <Knob
          key={id}
          {...dial}
          value={state.parameters[id]}
          defaultValue={defaultState.parameters[id]}
          max={id === "attackMs" ? 500 : 1}
          step={id === "attackMs" ? 1 : 0.001}
          scale={id === "attackMs" ? attackScale : undefined}
          format={id === "attackMs" ? milliseconds : id === "out" ? decibels : undefined}
          onChange={(value) => setParameter(id, value)}
          onGestureChange={(active) => setGesture(id, active)}
        />
      ))}
      <p className="sr-only" role="status">
        {connected ? "Connected" : "Preview"}
      </p>
    </InstrumentPanel>
  );
}

export default App;

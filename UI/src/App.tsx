import { BrandSignature } from "./components/BrandSignature";
import { ControlBank } from "./components/ControlBank";
import { GuitarTuner } from "./components/GuitarTuner";
import { InstrumentPanel } from "./components/InstrumentPanel";
import { Knob } from "./components/Knob";
import { MakerSignature } from "./components/MakerSignature";
import { usePluginState } from "./hooks/usePluginState";

function App() {
  const { state, connected, setParameter, setGesture } = usePluginState();

  return (
    <InstrumentPanel>
      <ControlBank
        side="left"
        parameters={state.parameters}
        onChange={setParameter}
        onGestureChange={setGesture}
      />
      <div className="center-column flex min-w-0 flex-col items-center">
        <GuitarTuner pitch={state.pitch} />
        <BrandSignature />
        <div className="character-control">
          <Knob
            label="Bow Character"
            description="Adds body, texture, and friction to the original signal."
            value={state.parameters.bowCharacter}
            defaultValue={0}
            compact
            onChange={(value) => setParameter("bowCharacter", value)}
            onGestureChange={(active) => setGesture("bowCharacter", active)}
          />
        </div>
        <MakerSignature connected={connected} />
      </div>
      <ControlBank
        side="right"
        parameters={state.parameters}
        onChange={setParameter}
        onGestureChange={setGesture}
      />
    </InstrumentPanel>
  );
}

export default App;

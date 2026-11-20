import * as styles from "./styles/panel.css";
import { BrandSignature } from "./components/BrandSignature";
import { ControlBank } from "./components/ControlBank";
import { GuitarTuner } from "./components/GuitarTuner";
import { InstrumentPanel } from "./components/InstrumentPanel";
import { Knob } from "./components/Knob";
import { MakerSignature } from "./components/MakerSignature";
import { defaultState } from "./types";
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
      <div className={`center-column flex min-w-0 flex-col items-center ${styles.center}`}>
        <GuitarTuner pitch={state.pitch} />
        <BrandSignature />
        <div className={`character-control ${styles.character}`}>
          <Knob
            label="Bow Character"
            description="Emphasize the woody midrange and soften the bright edge of the pickup."
            value={state.parameters.bowCharacter}
            defaultValue={defaultState.parameters.bowCharacter}
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

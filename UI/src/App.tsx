import { useCallback, useEffect, useMemo, useState } from "react";
import type { CSSProperties } from "react";
import { getPluginState, subscribeToState, updateParameter } from "./juce";
import {
  defaultState,
  type ParameterId,
  type PluginState,
} from "./types";

type SliderProps = {
  id: ParameterId;
  label: string;
  description: string;
  value: number;
  min: number;
  max: number;
  step: number;
  format: (value: number) => string;
  onChange: (id: ParameterId, value: number) => void;
};

function Slider({
  id,
  label,
  description,
  value,
  min,
  max,
  step,
  format,
  onChange,
}: SliderProps) {
  const percentage = ((value - min) / (max - min)) * 100;

  return (
    <label className="control-row">
      <span className="control-label">
        <span>
          <span className="block text-[13px] font-medium text-slate-100">{label}</span>
          <span className="mt-0.5 block text-[10px] uppercase tracking-[0.16em] text-slate-500">
            {description}
          </span>
        </span>
        <span className="font-mono text-xs text-cyan-300">{format(value)}</span>
      </span>
      <input
        aria-label={label}
        className="range-input"
        style={{ "--progress": `${percentage}%` } as CSSProperties}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(event) => onChange(id, Number(event.target.value))}
      />
    </label>
  );
}

function SignalBars({ confidence }: { confidence: number }) {
  const activeBars = Math.round(confidence * 12);

  return (
    <div className="flex h-8 items-end gap-1" aria-label={`Signal ${Math.round(confidence * 100)} percent`}>
      {Array.from({ length: 12 }, (_, index) => (
        <span
          className={`signal-bar ${index < activeBars ? "signal-bar-active" : ""}`}
          key={index}
          style={{ height: `${8 + (index % 4) * 5}px` }}
        />
      ))}
    </div>
  );
}

function App() {
  const [state, setState] = useState<PluginState>(defaultState);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    let mounted = true;

    getPluginState()
      .then((nextState) => {
        if (mounted) {
          setState(nextState);
          setConnected(true);
        }
      })
      .catch(() => setConnected(false));

    const unsubscribe = subscribeToState((nextState) => {
      if (mounted) {
        setState(nextState);
        setConnected(true);
      }
    });

    return () => {
      mounted = false;
      unsubscribe();
    };
  }, []);

  const setParameter = useCallback((id: ParameterId, value: number) => {
    setState((current) => ({
      ...current,
      parameters: { ...current.parameters, [id]: value },
    }));
    void updateParameter(id, value);
  }, []);

  const frequency = state.pitch.frequencyHz;
  const pitchText = state.pitch.note === "—" ? "Waiting" : state.pitch.note;
  const frequencyText = frequency > 0 ? `${frequency.toFixed(1)} Hz` : "No signal";
  const confidencePercent = Math.round(state.pitch.confidence * 100);

  const voices = useMemo(
    () =>
      state.voices.map((voice, index) => ({
        ...voice,
        label: `Voice ${index + 1}`,
      })),
    [state.voices],
  );

  return (
    <main className="app-shell">
      <header className="topbar">
        <div className="flex items-center gap-3">
          <div className="brand-mark" aria-hidden="true">
            <span />
            <span />
            <span />
          </div>
          <div>
            <p className="brand-name">ElectroBow</p>
            <p className="brand-caption">Acoustic intelligence / bowed synthesis</p>
          </div>
        </div>
        <div className="flex items-center gap-2 rounded-full border border-white/10 bg-white/[0.035] px-3 py-1.5 text-[10px] font-semibold uppercase tracking-[0.18em] text-slate-400">
          <span className={`status-dot ${connected ? "status-live" : ""}`} />
          {connected ? "Connected" : "Connecting"}
        </div>
      </header>

      <div className="content-grid">
        <section className="panel monitor-panel">
          <div className="section-kicker">Input monitor</div>
          <div className="monitor-ring">
            <div className="monitor-core">
              <span className="monitor-note">{pitchText}</span>
              <span className="monitor-frequency">{frequencyText}</span>
            </div>
          </div>
          <div className="monitor-footer">
            <div>
              <span className="metric-label">Tracking confidence</span>
              <span className="metric-value">{confidencePercent}%</span>
            </div>
            <SignalBars confidence={state.pitch.confidence} />
          </div>
          <div className="voices-block">
            <div className="flex items-center justify-between">
              <span className="section-kicker">Active voices</span>
              <span className="font-mono text-xs text-cyan-300">{voices.length}/4</span>
            </div>
            <div className="mt-3 grid grid-cols-2 gap-2">
              {voices.length > 0 ? (
                voices.map((voice) => (
                  <div className="voice-chip" key={`${voice.label}-${voice.frequencyHz}`}>
                    <span className="voice-chip-dot" />
                    <span>
                      <span className="block text-[10px] uppercase tracking-[0.14em] text-slate-500">
                        {voice.label}
                      </span>
                      <span className="font-mono text-xs text-slate-200">
                        {voice.frequencyHz.toFixed(1)} Hz
                      </span>
                    </span>
                  </div>
                ))
              ) : (
                <div className="col-span-2 rounded-lg border border-dashed border-white/10 px-3 py-4 text-center text-xs text-slate-500">
                  Play into the input to reveal tracked voices.
                </div>
              )}
            </div>
          </div>
        </section>

        <section className="panel controls-panel">
          <div className="flex items-start justify-between">
            <div>
              <div className="section-kicker">Instrument</div>
              <h1 className="mt-1 text-xl font-semibold tracking-tight text-white">Shape the bow</h1>
            </div>
            <div className="rounded-md border border-cyan-400/20 bg-cyan-400/10 px-2 py-1 text-[10px] font-semibold uppercase tracking-[0.16em] text-cyan-300">
              Live
            </div>
          </div>

          <div className="mt-6 space-y-5">
            <Slider
              id="bowPressure"
              label="Bow pressure"
              description="Contact weight"
              value={state.parameters.bowPressure}
              min={0}
              max={1}
              step={0.001}
              format={(value) => `${Math.round(value * 100)}%`}
              onChange={setParameter}
            />
            <Slider
              id="bowSpeed"
              label="Bow speed"
              description="Excitation velocity"
              value={state.parameters.bowSpeed}
              min={0}
              max={1}
              step={0.001}
              format={(value) => `${Math.round(value * 100)}%`}
              onChange={setParameter}
            />
            <Slider
              id="friction"
              label="Friction"
              description="String texture"
              value={state.parameters.friction}
              min={0}
              max={1}
              step={0.001}
              format={(value) => `${Math.round(value * 100)}%`}
              onChange={setParameter}
            />
            <div className="control-divider" />
            <Slider
              id="attackMs"
              label="Attack"
              description="Envelope rise"
              value={state.parameters.attackMs}
              min={0}
              max={500}
              step={1}
              format={(value) => `${Math.round(value)} ms`}
              onChange={setParameter}
            />
            <Slider
              id="naturalResonanceMs"
              label="Natural resonance"
              description="Release tail"
              value={state.parameters.naturalResonanceMs}
              min={0}
              max={1000}
              step={1}
              format={(value) => `${Math.round(value)} ms`}
              onChange={setParameter}
            />
          </div>
        </section>
      </div>

      <footer className="bottomline">
        <span>ANALYSIS ENGINE / POLYPHONIC YIN</span>
        <span className="text-slate-600">ELECTROBOW · 01</span>
      </footer>
    </main>
  );
}

export default App;

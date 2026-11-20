import { useId } from "react";
import { engravedRule } from "../styles/shared.css";
import * as styles from "../styles/tuner.css";
import type { PluginState } from "../types";
import { getTuning } from "../lib/pitch";
import { PanelScrew } from "./PanelScrew";

function point(angle: number, radius: number) {
  const radians = (angle * Math.PI) / 180;
  return { x: 180 + Math.sin(radians) * radius, y: 272 - Math.cos(radians) * radius };
}

const marks = Array.from({ length: 21 }, (_, index) => {
  const angle = -48 + index * 4.8;
  return {
    start: point(angle, 175),
    end: point(angle, index % 5 === 0 ? 197 : index % 2 === 0 ? 190 : 185),
  };
});

export function GuitarTuner({ pitch }: { pitch: PluginState["pitch"] }) {
  const id = useId();
  const tuning = getTuning(pitch.frequencyHz);
  const active = tuning !== null && pitch.note !== "—" && pitch.confidence > 0;
  const cents = active ? tuning.cents : 0;
  const note = active ? pitch.note.replace(/-?\d+$/, "") : "—";
  const frequency = active ? `${pitch.frequencyHz.toFixed(1)} Hz` : "No signal";

  return (
    <section className={`tuner w-full ${styles.tuner}`} aria-labelledby={`${id}-title`}>
      <h2
        id={`${id}-title`}
        className={`tuner-heading flex items-center justify-center whitespace-nowrap ${styles.heading}`}
      >
        <span className={`flex-1 ${engravedRule}`} aria-hidden="true" />
        Guitar Tuner
        <span className={`flex-1 ${engravedRule}`} aria-hidden="true" />
      </h2>
      <div className={`tuner-bezel relative ${styles.bezel}`}>
        <PanelScrew position="topLeft" variant="tuner" />
        <PanelScrew position="topRight" variant="tuner" />
        <PanelScrew position="bottomLeft" variant="tuner" />
        <PanelScrew position="bottomRight" variant="tuner" />
        <div className={`tuner-glass relative overflow-hidden ${styles.glass}`}>
          <svg
            viewBox="0 0 360 215"
            className={`tuner-scale block h-full w-full ${styles.scale}`}
            role="img"
            aria-label={
              active
                ? `${pitch.note}, ${frequency}, ${Math.round(cents)} cents`
                : "Tuner idle. Pitch tracking is not active in the current audio engine."
            }
          >
            <text x="180" y="43" className={`tuner-note ${styles.note}`} textAnchor="middle">
              {note}
            </text>
            <defs>
              <clipPath id={`${id}-needle-clip`}>
                <rect width="360" height="187" />
              </clipPath>
            </defs>
            <path d="M32 140 Q180 10 328 140" className={`meter-line ${styles.line}`} />
            <path
              d="M280 108 Q294 118 311 138 L328 140 Q315 123 299 112Z"
              fill="#ad3d2c"
              opacity=".85"
            />
            <path
              d="M50 155 Q180 33 310 155"
              className={`meter-line meter-line-fine ${styles.line} ${styles.fineLine}`}
            />
            {marks.map(({ start, end }, index) => (
              <line
                key={index}
                x1={start.x}
                y1={start.y}
                x2={end.x}
                y2={end.y}
                className={`meter-line meter-line-fine ${styles.line} ${styles.fineLine}`}
              />
            ))}
            <text x="43" y="170" textAnchor="middle" className={`tuner-limit ${styles.limit}`}>
              -50
            </text>
            <text x="317" y="170" textAnchor="middle" className={`tuner-limit ${styles.limit}`}>
              +50
            </text>
            <g clipPath={`url(#${id}-needle-clip)`}>
              <g
                className={`tuner-needle ${styles.needle}`}
                style={{ transform: `rotate(${cents * 0.96}deg)` }}
              >
                <path d="M175 272 180 74 185 272Z" fill="#27251e" />
                <path d="M180 272 180 79" stroke="#000" strokeOpacity=".2" strokeWidth="2" />
              </g>
            </g>
            <path
              d="M95 192 Q180 151 265 192"
              className={`meter-line meter-line-fine ${styles.line} ${styles.fineLine}`}
            />
            <text
              x="180"
              y="207"
              textAnchor="middle"
              className={`tuner-frequency ${styles.frequency}`}
            >
              {frequency}
            </text>
          </svg>
        </div>
      </div>
    </section>
  );
}

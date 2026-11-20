import { useId } from "react";
import { engravedRule } from "../styles/shared.css";
import * as styles from "../styles/tuner.css";
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

export function InputMeter({ level }: { level: number }) {
  const id = useId();
  const db = level > 0 ? 20 * Math.log10(level) : -Infinity;
  const display = db > -60 ? db.toFixed(1) : "−∞";
  const angle = -48 + 96 * Math.min(1, Math.max(0, (db + 60) / 60));

  return (
    <section className={`tuner w-full ${styles.tuner}`} aria-labelledby={`${id}-title`}>
      <h2
        id={`${id}-title`}
        className={`tuner-heading flex items-center justify-center whitespace-nowrap ${styles.heading}`}
      >
        <span className={`flex-1 ${engravedRule}`} aria-hidden="true" />
        Input Level
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
            aria-label={`Input level ${display} dBFS`}
          >
            <text x="180" y="43" className={`tuner-note ${styles.note}`} textAnchor="middle">
              {display}
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
              -60
            </text>
            <text x="317" y="170" textAnchor="middle" className={`tuner-limit ${styles.limit}`}>
              0
            </text>
            <g clipPath={`url(#${id}-needle-clip)`}>
              <g
                className={`tuner-needle ${styles.needle}`}
                style={{ transform: `rotate(${angle}deg)` }}
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
              dBFS
            </text>
          </svg>
        </div>
      </div>
    </section>
  );
}

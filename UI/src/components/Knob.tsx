import { useId, useRef, useState } from "react";
import type { PointerEvent } from "react";
import { knobGeometry } from "../styles/theme";
import * as styles from "../styles/knob.css";

type KnobProps = {
  label: string;
  value: number;
  min?: number;
  max?: number;
  step?: number;
  defaultValue?: number;
  description: string;
  disabled?: boolean;
  compact?: boolean;
  format?: (value: number) => string;
  onChange?: (value: number) => void;
  onGestureChange?: (active: boolean) => void;
};

const percent = (value: number) => `${Math.round(value * 100)}%`;
const ticks = Array.from(
  { length: knobGeometry.tickCount },
  (_, index) =>
    knobGeometry.startAngle + (index * knobGeometry.sweepAngle) / (knobGeometry.tickCount - 1),
);

export function Knob({
  label,
  value,
  min = 0,
  max = 1,
  step = 0.001,
  defaultValue = 0.5,
  description,
  disabled = false,
  compact = false,
  format = percent,
  onChange,
  onGestureChange,
}: KnobProps) {
  const id = useId();
  const drag = useRef<{ pointerId: number; y: number; value: number } | null>(null);
  const [dragging, setDragging] = useState(false);
  const boundedValue = Math.min(max, Math.max(min, value));
  const angle =
    knobGeometry.startAngle + ((boundedValue - min) / (max - min)) * knobGeometry.sweepAngle;

  function change(nextValue: number) {
    const rounded = min + Math.round((nextValue - min) / step) * step;
    onChange?.(Number(Math.min(max, Math.max(min, rounded)).toFixed(6)));
  }

  function startDrag(event: PointerEvent<HTMLInputElement>) {
    if (disabled || event.button !== 0 || drag.current) return;
    event.preventDefault();
    event.currentTarget.focus();
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { pointerId: event.pointerId, y: event.clientY, value: boundedValue };
    setDragging(true);
    onGestureChange?.(true);
  }

  function moveDrag(event: PointerEvent<HTMLInputElement>) {
    const current = drag.current;
    if (!current || current.pointerId !== event.pointerId) return;
    const sensitivity = (max - min) / (event.shiftKey ? 1800 : 180);
    current.value = Math.min(
      max,
      Math.max(min, current.value + (current.y - event.clientY) * sensitivity),
    );
    current.y = event.clientY;
    change(current.value);
  }

  function endDrag(event: PointerEvent<HTMLInputElement>) {
    if (drag.current?.pointerId !== event.pointerId) return;
    drag.current = null;
    setDragging(false);
    onGestureChange?.(false);
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  }

  return (
    <div
      className={`knob-control group relative flex flex-col items-center ${styles.control} ${compact ? styles.compact : ""}`}
      data-dragging={dragging || undefined}
      data-disabled={disabled || undefined}
      title={description}
    >
      <label className={`knob-label text-center ${styles.label}`} htmlFor={id}>
        {compact ? label : label.split(" ").join("\n")}
      </label>
      <div className={`knob-dial relative aspect-square ${styles.dial}`}>
        <div className="knob-ticks absolute inset-0" aria-hidden="true">
          {ticks.map((tick, index) => (
            <span
              key={tick}
              className={`${styles.tick} ${index % 5 === 0 ? styles.majorTick : ""}`}
              style={{ transform: `rotate(${tick}deg)` }}
            />
          ))}
        </div>
        <div className={`knob-rim absolute rounded-full ${styles.rim}`} aria-hidden="true">
          <div className={`knob-face absolute rounded-full ${styles.face}`} />
          <div
            className="knob-pointer absolute inset-0"
            style={{ transform: `rotate(${angle}deg)` }}
          >
            <span className={styles.pointer} />
          </div>
        </div>
        <input
          id={id}
          type="range"
          className={`knob-input absolute inset-0 h-full w-full opacity-0 ${styles.input}`}
          aria-label={label}
          aria-describedby={`${id}-description`}
          aria-valuetext={
            disabled ? "Unavailable in the current audio engine" : format(boundedValue)
          }
          min={min}
          max={max}
          step={step}
          value={boundedValue}
          disabled={disabled}
          onChange={(event) => change(Number(event.target.value))}
          onPointerDown={startDrag}
          onPointerMove={moveDrag}
          onPointerUp={endDrag}
          onPointerCancel={endDrag}
          onLostPointerCapture={endDrag}
          onDoubleClick={() => change(defaultValue)}
        />
      </div>
      <output
        className={`knob-value absolute text-center ${styles.value}`}
        htmlFor={id}
        aria-hidden="true"
      >
        {disabled ? "Not active" : format(boundedValue)}
      </output>
      <span id={`${id}-description`} className="sr-only">
        {description}
        {!disabled &&
          " Drag up or down. Hold Shift for fine adjustment. Use arrow keys or double-click to reset."}
      </span>
    </div>
  );
}

import { useId, useRef, useState } from "react";
import type { PointerEvent } from "react";
import { plate, sweep } from "../styles/theme";
import * as styles from "../styles/knob.css";

export type DialPosition = { x: number; y: number; diameter: number };
type KnobProps = {
  label: string;
  value: number;
  position: DialPosition;
  scale?: { toPosition: (value: number) => number; fromPosition: (position: number) => number };
  max?: number;
  step?: number;
  defaultValue?: number;
  description: string;
  format?: (value: number) => string;
  onChange?: (value: number) => void;
  onGestureChange?: (active: boolean) => void;
};

const percent = (value: number) => `${Math.round(value * 100)}%`;
export function Knob({
  label,
  value,
  position: { x, y, diameter },
  scale,
  max = 1,
  step = 0.001,
  defaultValue = 0.5,
  description,
  format = percent,
  onChange,
  onGestureChange,
}: KnobProps) {
  const id = useId();
  const drag = useRef<{ pointerId: number; y: number; value: number } | null>(null);
  const [dragging, setDragging] = useState(false);
  const boundedValue = Math.min(max, Math.max(0, value));
  const fraction = scale ? scale.toPosition(boundedValue) : boundedValue / max;
  const angle = (fraction - 0.5) * sweep;

  function change(nextValue: number) {
    const rounded = Math.round(nextValue / step) * step;
    onChange?.(Number(Math.min(max, Math.max(0, rounded)).toFixed(6)));
  }

  function startDrag(event: PointerEvent<HTMLInputElement>) {
    if (event.button !== 0 || drag.current) return;
    event.preventDefault();
    event.currentTarget.focus();
    event.currentTarget.setPointerCapture(event.pointerId);
    drag.current = { pointerId: event.pointerId, y: event.clientY, value: fraction };
    setDragging(true);
    onGestureChange?.(true);
  }

  function moveDrag(event: PointerEvent<HTMLInputElement>) {
    const current = drag.current;
    if (!current || current.pointerId !== event.pointerId) return;
    const sensitivity = 1 / (event.shiftKey ? 1800 : 180);
    current.value = Math.min(
      1,
      Math.max(0, current.value + (current.y - event.clientY) * sensitivity),
    );
    current.y = event.clientY;
    change(scale ? scale.fromPosition(current.value) : current.value * max);
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
      className={styles.control}
      style={{
        left: `${(100 * x) / plate.width}%`,
        top: `${(100 * y) / plate.height}%`,
        width: `${(100 * diameter) / plate.width}%`,
      }}
      data-dragging={dragging || undefined}
      title={description}
    >
      <div className={styles.dial}>
        <div
          className={styles.rotor}
          aria-hidden="true"
          style={{
            // Register the rotating face to the same pixels as the fixed engraving.
            backgroundSize: `${(100 * plate.width) / diameter}% ${(100 * plate.height) / diameter}%`,
            backgroundPosition: `${(100 * (x - diameter / 2)) / (plate.width - diameter)}% ${(100 * (y - diameter / 2)) / (plate.height - diameter)}%`,
            transform: `rotate(${angle}deg)`,
          }}
        />
        <input
          id={id}
          type="range"
          className={styles.input}
          aria-label={label}
          aria-describedby={`${id}-description`}
          aria-valuetext={format(boundedValue)}
          min={0}
          max={max}
          step={step}
          value={boundedValue}
          onChange={(event) => change(Number(event.target.value))}
          onPointerDown={startDrag}
          onPointerMove={moveDrag}
          onPointerUp={endDrag}
          onPointerCancel={endDrag}
          onLostPointerCapture={endDrag}
          onDoubleClick={() => change(defaultValue)}
        />
      </div>
      <output className={styles.value} htmlFor={id} aria-hidden="true">
        {format(boundedValue)}
      </output>
      <span id={`${id}-description`} className="sr-only">
        {description}
        {
          " Drag up or down. Hold Shift for fine adjustment. Use arrow keys to adjust or double-click to reset."
        }
      </span>
    </div>
  );
}

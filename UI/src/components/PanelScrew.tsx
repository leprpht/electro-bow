import { positions, screw, tunerScrew } from "../styles/screw.css";
import type { ScrewPosition, ScrewVariant } from "../styles/screw.css";

type PanelScrewProps = {
  position: ScrewPosition;
  variant?: ScrewVariant;
};

export function PanelScrew({ position, variant = "panel" }: PanelScrewProps) {
  return (
    <span
      className={`absolute rounded-full ${screw} ${positions[variant][position]} ${variant === "tuner" ? tunerScrew : ""}`}
      aria-hidden="true"
    />
  );
}

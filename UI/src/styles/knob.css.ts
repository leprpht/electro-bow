import { style } from "@vanilla-extract/css";

export const control = style({
  position: "absolute",
  transform: "translate(-50%, -50%)",
  aspectRatio: "1",
});
export const rotor = style({
  position: "absolute",
  inset: 0,
  borderRadius: "50%",
  backgroundImage: "var(--faceplate)",
  backgroundRepeat: "no-repeat",
  pointerEvents: "none",
});
export const input = style({
  position: "absolute",
  top: "50%",
  left: "50%",
  transform: "translate(-50%, -50%)",
  width: "max(100%, 44px)",
  height: "max(100%, 44px)",
  opacity: 0,
  cursor: "ns-resize",
  touchAction: "none",
  margin: 0,
});
export const dial = style({
  height: "100%",
  borderRadius: "50%",
  selectors: {
    "&:focus-within": { outline: "2px solid #f2d5a8", outlineOffset: "3px" },
  },
});
export const value = style({
  position: "absolute",
  top: "105%",
  left: "50%",
  transform: "translateX(-50%)",
  padding: "1px 0.45em",
  borderRadius: "3px",
  color: "#f6e4c8",
  background: "#171412",
  fontSize: "clamp(12px, 1.65cqw, 22px)",
  lineHeight: 1.2,
  whiteSpace: "nowrap",
  pointerEvents: "none",
  opacity: 0,
  selectors: {
    [`${control}:hover &, ${control}:focus-within &, ${control}[data-dragging] &`]: { opacity: 1 },
  },
});

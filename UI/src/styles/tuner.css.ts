import { style } from "@vanilla-extract/css";
import { media, metalTexture } from "./theme";

export const tuner = style({
  width: "85%",
  "@media": {
    [media.mobile]: {
      width: "100%",
    },
  },
});

export const heading = style({
  gap: "1.5cqw",
  fontSize: "2.65cqw",
  letterSpacing: "0.025em",
  margin: "0 0 1.65cqw",
  lineHeight: 1.3,
  textShadow: "0 2px 2px #000",
  "@media": {
    [media.mobile]: {
      fontSize: "3.5cqw",
      gap: "1cqw",
    },
  },
});

export const bezel = style({
  padding: "2.5cqw 2.1cqw",
  border: "0.18cqw solid #b1ab99",
  borderRadius: "1.3cqw",
  background: `${metalTexture}, linear-gradient(130deg, #4b4940, #2e302b 40%, #44433b 75%, #292b26)`,
  boxShadow:
    "0 0.5cqw 0.8cqw #000, 0 0 0 0.22cqw #080b08, inset 0 0 0 0.22cqw #34352e, inset 0 0 0 0.3cqw #817a69",
  "@media": {
    [media.mobile]: {
      marginTop: "3cqw",
    },
  },
});

export const glass = style({
  aspectRatio: "360 / 215",
  border: "0.22cqw solid #15160f",
  borderRadius: "45% 45% 8% 8% / 11% 11% 6% 6%",
  background: `${metalTexture}, radial-gradient(ellipse at 50% 70%, #e9d8ae 0, #cfbb90 70%, #96815d 100%)`,
  boxShadow: "0 0 0 0.13cqw #9d937b, inset 0 0.7cqw 1cqw #0009, inset 0 0 1.8cqw #31291d7a",
  "::after": {
    content: '""',
    position: "absolute",
    inset: "0",
    pointerEvents: "none",
    background: "linear-gradient(130deg, #fff1, transparent 45%, #0001)",
    boxShadow: "inset 0 1cqw 1.1cqw #0008",
  },
});

export const scale = style({
  fill: "#25241c",
});

export const line = style({
  fill: "none",
  stroke: "#302c22",
  strokeWidth: 2,
});

export const fineLine = style({
  strokeWidth: 1.3,
});

export const note = style({
  font: "500 37px var(--font-sans)",
});

export const limit = style({
  font: "500 21px var(--font-sans)",
});

export const frequency = style({
  font: "500 27px var(--font-sans)",
});

export const needle = style({
  transformOrigin: "180px 272px",
  transition: "transform 150ms ease-out",
  filter: "drop-shadow(2px 1px 1px #0004)",
});

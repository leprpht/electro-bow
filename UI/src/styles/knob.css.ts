import { style } from "@vanilla-extract/css";
import { media } from "./theme";

export const control = style({});
export const compact = style({});

export const label = style({
  display: "flex",
  alignItems: "end",
  justifyContent: "center",
  maxWidth: "11cqw",
  whiteSpace: "pre-line",
  minHeight: "5.3cqw",
  fontSize: "2.7cqw",
  fontWeight: 500,
  lineHeight: 0.98,
  letterSpacing: "0.025em",
  textShadow: "0 1px 1px #000",
  "@media": {
    [media.mobile]: {
      fontSize: "3.8cqw",
      maxWidth: "16cqw",
      minHeight: "8cqw",
    },
  },
  selectors: {
    [`${compact} &`]: {
      minHeight: "0",
      maxWidth: "none",
      fontSize: "max(1.55cqw, 12px)",
      letterSpacing: "0.08em",
      "@media": {
        [media.mobile]: {
          fontSize: "2.7cqw",
        },
      },
    },
    [`${control}[data-disabled] &`]: {
      color: "#bdaf94",
    },
  },
});

export const dial = style({
  width: "17.2cqw",
  marginTop: "0.65cqw",
  "@media": {
    [media.mobile]: {
      marginTop: "2cqw",
    },
  },
  selectors: {
    [`${compact} &`]: {
      width: "8cqw",
      marginTop: "0.7cqw",
      "@media": {
        [media.mobile]: {
          width: "max(12cqw, 44px)",
          marginTop: "2cqw",
        },
      },
    },
    "&:has(:focus-visible)": {
      outline: "2px solid #e0c28b",
      outlineOffset: "3px",
      borderRadius: "50%",
    },
  },
});

export const tick = style({
  position: "absolute",
  left: "calc(50% - 0.065cqw)",
  top: "0",
  width: "0.13cqw",
  height: "0.8cqw",
  transformOrigin: "50% 8.6cqw",
  background: "#c7b996",
  boxShadow: "0 1px 1px #000",
  selectors: {
    [`${compact} &`]: {
      transformOrigin: "50% 4cqw",
      height: "0.45cqw",
      "@media": {
        [media.mobile]: {
          transformOrigin: "50% max(6cqw, 22px)",
        },
      },
    },
  },
});

export const majorTick = style({
  width: "0.22cqw",
  height: "1.1cqw",
  selectors: {
    [`${compact} &`]: {
      height: "0.55cqw",
    },
  },
});

export const rim = style({
  inset: "10%",
  border: "0.15cqw solid #100f0d",
  background: "repeating-conic-gradient(#c2bca9 0deg 1deg, #4d4b43 1deg 2deg, #a09c8d 2deg 3deg)",
  boxShadow:
    "0 1.4cqw 1.5cqw -0.25cqw #000d, 0 0.3cqw 0.4cqw #000, 0 0 0 0.19cqw #090b09, 0 0 0 0.32cqw #6f6755",
});

export const face = style({
  inset: "3%",
  border: "0.16cqw solid #b0a58a",
  background:
    "repeating-radial-gradient( circle at center, transparent 0 1px, #ffffff04 1px 2px, transparent 2px 3px ), conic-gradient( from 25deg, #151716, #292b28 35deg, #0d100f 85deg, #373936 145deg, #131513 190deg, #0b0e0c 245deg, #3b3e39 300deg, #151716 )",
  boxShadow:
    "inset 0 0 0 0.25cqw #131613, inset 0 0.25cqw 0.4cqw #e5ddbb24, inset 0 -0.3cqw 0.6cqw #000",
});

export const pointer = style({
  position: "absolute",
  left: "calc(50% - 0.35cqw)",
  top: "2%",
  width: "0.7cqw",
  height: "31%",
  border: "1px solid #faf0d7",
  borderRadius: "0.2cqw",
  background: "linear-gradient(90deg, #bdb294, #eee0bb 50%, #c9b995)",
  boxShadow: "0 0 2px #000, 1px 2px 2px #0007",
  selectors: {
    [`${compact} &`]: {
      width: "0.4cqw",
      left: "calc(50% - 0.2cqw)",
    },
  },
});

export const input = style({
  cursor: "ns-resize",
  touchAction: "none",
  ":disabled": {
    cursor: "not-allowed",
  },
});

export const value = style({
  bottom: "-1.6cqw",
  fontSize: "max(1.2cqw, 11px)",
  letterSpacing: "0.07em",
  opacity: 0,
  transition: "opacity 120ms",
  "@media": {
    [media.mobile]: {
      fontSize: "2.5cqw",
      bottom: "-4cqw",
    },
  },
  selectors: {
    [`${compact} &`]: {
      bottom: "-1.4cqw",
      fontSize: "1.1cqw",
      "@media": {
        [media.mobile]: {
          fontSize: "2.5cqw",
          bottom: "-3.5cqw",
        },
      },
    },
    [`${control}:hover &, ${control}:focus-within &, ${control}[data-dragging] &`]: {
      opacity: 1,
    },
  },
});

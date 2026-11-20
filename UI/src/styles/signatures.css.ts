import { style } from "@vanilla-extract/css";
import { media } from "./theme";

export const brand = style({
  width: "118%",
  margin: "6cqw -9% 0",
  font: "400 8.8cqw/0.95 var(--font-script)",
  letterSpacing: "-0.045em",
  transform: "rotate(-12deg)",
  textShadow: "0 2px 0 #0007",
  "@media": {
    [media.mobile]: {
      marginTop: "18cqw",
      fontSize: "8cqw",
    },
  },
});

export const lightning = style({
  bottom: "-3.2cqw",
  height: "5cqw",
  filter: "drop-shadow(0 1px 0 #0008)",
});

export const maker = style({
  width: "110%",
  marginTop: "auto",
  paddingTop: "2.1cqw",
  "@media": {
    [media.mobile]: {
      width: "130%",
      paddingTop: "6cqw",
    },
  },
});

export const signature = style({
  gap: "1.7cqw",
  font: "400 3.05cqw/1.2 var(--font-script)",
  "@media": {
    [media.mobile]: {
      fontSize: "4cqw",
      gap: "1cqw",
    },
  },
});

export const by = style({
  fontSize: "2.3cqw",
  "@media": {
    [media.mobile]: {
      fontSize: "3cqw",
    },
  },
});

export const status = style({
  marginTop: "0.8cqw",
  color: "#a79d86",
  font: "500 max(0.95cqw, 9px)/1.3 var(--font-sans)",
  letterSpacing: "0.1em",
  "@media": {
    [media.mobile]: {
      fontSize: "max(1.8cqw, 8px)",
      letterSpacing: "0.02em",
      marginTop: "2cqw",
    },
  },
});

export const dot = style({
  width: "0.4cqw",
  height: "0.4cqw",
  background: "#8d8062",
});

export const live = style({
  background: "#b4c58b",
  boxShadow: "0 0 4px #b4c58b66",
});

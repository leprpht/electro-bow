import { globalStyle, style } from "@vanilla-extract/css";
import { media, metalTexture, panelAspectRatio } from "./theme";

export const shell = style({
  containerType: "inline-size",
  width: `min(100%, calc((100svh - 2.4vmin) * ${panelAspectRatio}))`,
  // Explicit border-box height preserves WebKit sizing with a 100%-height child.
  height: `min(calc((100vw - 2.4vmin) / ${panelAspectRatio}), calc(100svh - 2.4vmin))`,
  aspectRatio: panelAspectRatio,
  padding: "3.4% 6.3% 3.1%",
  border: "3px solid #57544a",
  borderRadius: "2.7cqw",
  background: "#242522",
  boxShadow:
    "0 12px 20px #000b, 0 2px 3px #000, inset 0 0 0 2px #1a1b18, inset 0 0 0 5px #777164, inset 0 0 0 7px #191b19",
  "@media": {
    [media.mobile]: {
      width: "100%",
      height: "max(420px, 110vw)",
      minWidth: "0",
      aspectRatio: "auto",
      padding: "28px 20px 24px",
      borderRadius: "16px",
    },
  },
});

export const surface = style({
  zIndex: -1,
  borderRadius: "inherit",
  background: `${metalTexture}, repeating-linear-gradient(0deg, #ffffff02 0 1px, transparent 1px 4px), radial-gradient(ellipse at 45% 0, #ffffff07, transparent 65%), linear-gradient(115deg, #292a27, #171a18 50%, #22231f)`,
  "::after": {
    content: '""',
    position: "absolute",
    inset: "1%",
    border: "1px solid #ac9a79",
    borderRadius: "1.8cqw",
    boxShadow: "0 0 0 2px #101210, inset 0 0 10px #0008",
    opacity: 0.65,
  },
});

export const layout = style({
  gridTemplateColumns: "23% 46% 23%",
  gap: "4%",
  "@media": {
    [media.mobile]: {
      gridTemplateColumns: "25% 44% 25%",
      gap: "3%",
    },
  },
});

export const bank = style({
  padding: "0 0 2.5cqw",
  "@media": {
    [media.mobile]: {
      paddingTop: "7cqw",
      paddingBottom: "10cqw",
    },
  },
});

export const center = style({
  paddingTop: "0.3cqw",
});

export const character = style({
  marginTop: "auto",
  paddingTop: "5cqw",
  "@media": {
    [media.mobile]: {
      paddingTop: "12cqw",
    },
  },
});

// Keep base and responsive root sizing together so stylesheet load order cannot
// override the mobile padding. Share the breakpoint with the component styles.
globalStyle("#root", {
  display: "grid",
  placeItems: "center",
  gridTemplateColumns: "minmax(0, 1fr)",
  minHeight: "100svh",
  padding: "1.2vmin",
  "@media": { [media.mobile]: { padding: 6 } },
});

import { globalStyle, style } from "@vanilla-extract/css";
import { plate } from "./theme";

globalStyle("#root", {
  height: "100vh",
  display: "grid",
  placeItems: "center",
});

export const shell = style({
  position: "relative",
  width: `min(100%, ${(plate.width / plate.height) * 100}vh)`,
  aspectRatio: `${plate.width} / ${plate.height}`,
  containerType: "inline-size",
  userSelect: "none",
});

export const artwork = style({
  display: "block",
  width: "100%",
  height: "100%",
  pointerEvents: "none",
});

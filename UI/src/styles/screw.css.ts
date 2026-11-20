import { style, styleVariants } from "@vanilla-extract/css";

export const screw = style({
  width: "3cqw",
  aspectRatio: "1",
  zIndex: 1,
  border: "1px solid #151612",
  background:
    "radial-gradient(circle at 38% 25%, #d5cbb4 0, #8e8777 25%, #48473e 60%, #1b1c17 85%)",
  boxShadow: "0 2px 3px #000, 0 0 0 2px #1119, inset 0 1px 2px #eee5",
  "::after": {
    content: '""',
    position: "absolute",
    inset: "28%",
    background: "#10120f",
    clipPath:
      "polygon( 35% 0, 65% 0, 65% 35%, 100% 35%, 100% 65%, 65% 65%, 65% 100%, 35% 100%, 35% 65%, 0 65%, 0 35%, 35% 35% )",
    transform: "rotate(8deg)",
    filter: "drop-shadow(0 1px 0 #dfd0ad)",
  },
});
export const tunerScrew = style({ width: "1.8cqw" });
export const positions = {
  panel: styleVariants({
    topLeft: {
      left: "1.7%",
      top: "2.2%",
    },
    topRight: {
      right: "1.7%",
      top: "2.2%",
      transform: "rotate(12deg)",
    },
    bottomLeft: {
      left: "1.7%",
      bottom: "2.2%",
      transform: "rotate(-15deg)",
    },
    bottomRight: {
      right: "1.7%",
      bottom: "2.2%",
      transform: "rotate(25deg)",
    },
  }),
  tuner: styleVariants({
    topLeft: {
      left: "2.4%",
      top: "3.5%",
    },
    topRight: {
      right: "2.4%",
      top: "3.5%",
      transform: "rotate(12deg)",
    },
    bottomLeft: {
      left: "2.4%",
      bottom: "3.5%",
      transform: "rotate(-15deg)",
    },
    bottomRight: {
      right: "2.4%",
      bottom: "3.5%",
      transform: "rotate(25deg)",
    },
  }),
};
export type ScrewVariant = keyof typeof positions;
export type ScrewPosition = keyof typeof positions.panel;

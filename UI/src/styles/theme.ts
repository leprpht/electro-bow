import metalGrain from "../assets/metal-grain.svg";

export const media = { mobile: "(max-width: 600px)" } as const;
export const metalTexture = `url("${metalGrain}")`;
export const panelAspectRatio = 1.38;

export const knobGeometry = { tickCount: 11, startAngle: -135, sweepAngle: 270 } as const;

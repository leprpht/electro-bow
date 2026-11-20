export function getTuning(frequencyHz: number) {
  if (!Number.isFinite(frequencyHz) || frequencyHz <= 0) return null;
  const midi = 69 + 12 * Math.log2(frequencyHz / 440);
  return { cents: (midi - Math.round(midi)) * 100 };
}

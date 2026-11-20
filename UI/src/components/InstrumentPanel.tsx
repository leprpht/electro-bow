import type { ReactNode } from "react";
import { PanelScrew } from "./PanelScrew";

export function InstrumentPanel({ children }: { children: ReactNode }) {
  return (
    <main className="app-shell relative isolate" aria-label="Electro-Bow instrument">
      <div className="panel-surface absolute inset-0 pointer-events-none" aria-hidden="true" />
      <PanelScrew className="screw-top-left" />
      <PanelScrew className="screw-top-right" />
      <PanelScrew className="screw-bottom-left" />
      <PanelScrew className="screw-bottom-right" />
      <div className="instrument-layout relative grid h-full">{children}</div>
    </main>
  );
}

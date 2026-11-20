import type { ReactNode } from "react";
import * as styles from "../styles/panel.css";
import { PanelScrew } from "./PanelScrew";

export function InstrumentPanel({ children }: { children: ReactNode }) {
  return (
    <main
      className={`app-shell relative isolate ${styles.shell}`}
      aria-label="Electro-Bow instrument"
    >
      <div
        className={`panel-surface absolute inset-0 pointer-events-none ${styles.surface}`}
        aria-hidden="true"
      />
      <PanelScrew position="topLeft" />
      <PanelScrew position="topRight" />
      <PanelScrew position="bottomLeft" />
      <PanelScrew position="bottomRight" />
      <div className={`instrument-layout relative grid h-full ${styles.layout}`}>{children}</div>
    </main>
  );
}

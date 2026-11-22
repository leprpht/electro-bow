import type { CSSProperties, ReactNode } from "react";
import faceplate from "../assets/electro-bow-faceplate.jpg";
import * as styles from "../styles/panel.css";

export function InstrumentPanel({ children }: { children: ReactNode }) {
  return (
    <main
      className={`app-shell ${styles.shell}`}
      aria-label="Electro-Bow instrument"
      style={{ "--faceplate": `url("${faceplate}")` } as CSSProperties}
    >
      <img
        className={styles.artwork}
        src={faceplate}
        alt="Electro-Bow by Unholy People"
        draggable={false}
      />
      {children}
    </main>
  );
}

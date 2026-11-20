import { engravedRule } from "../styles/shared.css";
import * as styles from "../styles/signatures.css";
export function MakerSignature({ connected }: { connected: boolean }) {
  return (
    <footer className={`maker-signature flex flex-col items-center ${styles.maker}`}>
      <p
        className={`flex w-full items-center justify-center whitespace-nowrap ${styles.signature}`}
      >
        <span className={`flex-1 ${engravedRule}`} aria-hidden="true" />
        <span>
          <span className={`maker-by ${styles.by}`}>by</span> Unholy People
        </span>
        <span className={`flex-1 ${engravedRule}`} aria-hidden="true" />
      </p>
      <p className={`connection-status flex items-center gap-1.5 ${styles.status}`} role="status">
        <span
          className={`status-dot rounded-full ${styles.dot} ${connected ? `status-live ${styles.live}` : ""}`}
          aria-hidden="true"
        />
        {connected ? "Connected" : "Preview"} · Attack & Character active
      </p>
    </footer>
  );
}

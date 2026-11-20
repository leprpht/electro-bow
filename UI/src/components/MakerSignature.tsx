export function MakerSignature({ connected }: { connected: boolean }) {
  return (
    <footer className="maker-signature flex flex-col items-center">
      <p className="flex w-full items-center justify-center whitespace-nowrap">
        <span className="engraved-rule flex-1" aria-hidden="true" />
        <span>
          <span className="maker-by">by</span> Unholy People
        </span>
        <span className="engraved-rule flex-1" aria-hidden="true" />
      </p>
      <p className="connection-status flex items-center gap-1.5" role="status">
        <span
          className={`status-dot rounded-full ${connected ? "status-live" : ""}`}
          aria-hidden="true"
        />
        {connected ? "Connected" : "Preview"} · Attack & Character active
      </p>
    </footer>
  );
}

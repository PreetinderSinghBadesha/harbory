import { useState } from "react";

export function CopyButton({ text, label }: { text: string; label: string }) {
  const [copied, setCopied] = useState(false);

  async function handleCopy() {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Clipboard access can be denied by the browser; the command is
      // still visible and selectable, so this is a silent no-op.
    }
  }

  return (
    <button
      type="button"
      onClick={handleCopy}
      aria-label={`Copy ${label}`}
      style={{ flexShrink: 0, display: "inline-flex", background: "none", border: "none", color: "var(--muted)", cursor: "pointer", padding: 2 }}
    >
      {copied ? (
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="var(--hp-dark)" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <path d="M20 6 9 17l-5-5" />
        </svg>
      ) : (
        <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
          <rect x="8" y="8" width="12" height="12" rx="2" />
          <path d="M4 16V5a1 1 0 0 1 1-1h11" />
        </svg>
      )}
    </button>
  );
}

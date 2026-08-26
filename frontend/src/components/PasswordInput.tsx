import { useState, type InputHTMLAttributes } from "react";

function EyeIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M1.5 12S5 5 12 5s10.5 7 10.5 7-3.5 7-10.5 7S1.5 12 1.5 12Z" />
      <circle cx="12" cy="12" r="3" />
    </svg>
  );
}

function EyeOffIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d="M3 3l18 18" />
      <path d="M10.6 5.1A10.9 10.9 0 0 1 12 5c7 0 10.5 7 10.5 7a13.6 13.6 0 0 1-3.1 3.9M6.6 6.6C3.4 8.6 1.5 12 1.5 12S5 19 12 19a10.7 10.7 0 0 0 4.2-.85" />
      <path d="M9.9 9.9a3 3 0 0 0 4.2 4.2" />
    </svg>
  );
}

/** A password `<input>` with a show/hide toggle — same visual footing as
 * every other input in the game-HUD system (this just wraps one, it
 * doesn't reimplement the styling), plus an icon button absolutely
 * positioned inside it. `containerStyle` is for the wrapper div itself —
 * needed e.g. inside `.inline-form`, whose `input { flex: 1 1 8rem }`
 * rule would otherwise size the inner input but leave this wrapper at
 * its flex default. */
export function PasswordInput({
  containerStyle,
  ...props
}: Omit<InputHTMLAttributes<HTMLInputElement>, "type"> & { containerStyle?: React.CSSProperties }) {
  const [visible, setVisible] = useState(false);

  return (
    <div style={{ position: "relative", ...containerStyle }}>
      <input {...props} type={visible ? "text" : "password"} style={{ width: "100%", paddingRight: 38, ...props.style }} />
      <button
        type="button"
        onClick={() => setVisible((v) => !v)}
        aria-label={visible ? "Hide password" : "Show password"}
        tabIndex={-1}
        style={{
          position: "absolute",
          right: 4,
          top: "50%",
          transform: "translateY(-50%)",
          background: "none",
          border: "none",
          cursor: "pointer",
          padding: 6,
          display: "flex",
          color: "var(--muted)",
        }}
      >
        {visible ? <EyeOffIcon /> : <EyeIcon />}
      </button>
    </div>
  );
}

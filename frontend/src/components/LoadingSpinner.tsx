/** Three bouncing pixel squares — the app's one loading indicator, used
 * anywhere a "Loading…" text block or a pending button currently just
 * shows an ellipsis. `label`, if given, renders after the dots in the
 * same muted mono style already used for that copy throughout the app.
 * `dotColor` defaults to the clay accent, which reads fine on the
 * cream/white backgrounds this is normally used against — override it
 * (e.g. `var(--panel)`) when dropping this onto a clay-colored primary
 * button, where clay-on-clay dots would be nearly invisible. */
export function LoadingSpinner({ label, dotColor }: { label?: string; dotColor?: string }) {
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: 10 }}>
      <span className="pixel-spinner" aria-hidden="true" style={dotColor ? ({ "--pixel-spinner-color": dotColor } as React.CSSProperties) : undefined}>
        <span />
        <span />
        <span />
      </span>
      {label && (
        <span className="mono" style={{ fontSize: 12, color: "var(--muted)" }}>
          {label}
        </span>
      )}
    </span>
  );
}

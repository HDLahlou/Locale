import type { ReactNode } from "react";

export type Status = "ok" | "warn" | "error";

export interface StatusLabelProps {
  status: Status;
  /** The word shown, for example "Live", "Waiting" or "Not sent". Required: colour is never the only signal. */
  children: ReactNode;
}

const ICONS: Record<Status, string> = { ok: "●", warn: "▲", error: "✕" };

export function StatusLabel({ status, children }: StatusLabelProps) {
  return (
    <span className={`lc-status lc-status--${status}`} data-status={status}>
      <span className="lc-status__icon" aria-hidden="true">
        {ICONS[status]}
      </span>
      <span className="lc-status__text">{children}</span>
    </span>
  );
}

import { labels, roleLabel } from "./labels";
import type { Role } from "./labels";

export type BadgeProps = { kind: "verified"; label?: string } | { kind: "role"; role: Role };

export function Badge(props: BadgeProps) {
  if (props.kind === "verified") {
    const label = props.label ?? labels.verified;
    return (
      <span className="lc-badge lc-badge--verified" role="img" aria-label={label} title={label}>
        <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false">
          <circle cx="8" cy="8" r="8" fill="currentColor" />
          <path
            d="M4.5 8.3l2.3 2.3 4.7-4.9"
            fill="none"
            stroke="var(--lc-badge-tint)"
            strokeWidth="1.8"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </span>
    );
  }
  return <span className="lc-badge lc-badge--role">{roleLabel(props.role)}</span>;
}

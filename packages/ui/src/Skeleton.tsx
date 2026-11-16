import type { CSSProperties } from "react";

export interface SkeletonProps {
  width?: CSSProperties["width"];
  height?: CSSProperties["height"];
  /** Fully round, for avatars. */
  circle?: boolean;
}

/** A loading placeholder. The shimmer stops under prefers-reduced-motion (see components.css). */
export function Skeleton({ width, height = 16, circle = false }: SkeletonProps) {
  return (
    <span
      className={circle ? "lc-skeleton lc-skeleton--circle" : "lc-skeleton"}
      style={{ width, height }}
      aria-hidden="true"
      data-testid="skeleton"
    />
  );
}

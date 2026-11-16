import type { ButtonHTMLAttributes } from "react";

export interface PillProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "aria-pressed"> {
  selected?: boolean;
}

/** A selectable pill, for the people row. A toggle button, so Space and Enter work. */
export function Pill({
  selected = false,
  type = "button",
  className,
  children,
  ...rest
}: PillProps) {
  const classes = ["lc-pill", className].filter(Boolean).join(" ");
  return (
    <button {...rest} type={type} className={classes} aria-pressed={selected}>
      {children}
    </button>
  );
}

import type { ButtonHTMLAttributes } from "react";

export type ButtonVariant = "primary" | "secondary" | "quiet" | "destructive";

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  /** Shows a busy indicator and ignores clicks while true. */
  busy?: boolean;
}

export function Button({
  variant = "primary",
  busy = false,
  disabled = false,
  type = "button",
  className,
  children,
  onClick,
  ...rest
}: ButtonProps) {
  const classes = ["lc-button", `lc-button--${variant}`, className].filter(Boolean).join(" ");
  return (
    <button
      {...rest}
      type={type}
      className={classes}
      disabled={disabled}
      aria-busy={busy || undefined}
      aria-disabled={busy || undefined}
      data-busy={busy || undefined}
      onClick={busy ? undefined : onClick}
    >
      {busy ? <span className="lc-button__spinner" aria-hidden="true" /> : null}
      {children}
    </button>
  );
}

import clsx from "clsx";
import { type ComponentPropsWithoutRef } from "react";

type Color = "accent" | "red" | "zinc";

type ButtonProps = {
  color?: Color;
  outline?: boolean;
  plain?: boolean;
  className?: string;
  children: React.ReactNode;
} & ComponentPropsWithoutRef<"button">;

const base =
  "relative inline-flex cursor-pointer items-center justify-center gap-1.5 rounded px-3 py-1.5 text-xs font-semibold transition-colors " +
  "focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--color-canvas)] " +
  "disabled:cursor-not-allowed disabled:opacity-40";

const styles: Record<string, string> = {
  "solid/accent":
    "bg-[var(--color-accent)] text-[#080818] hover:opacity-90 active:opacity-80",
  "solid/zinc":
    "bg-[var(--color-surface)] text-[var(--color-text)] border border-[var(--color-border)] hover:border-[var(--color-accent)]",
  "solid/red":
    "bg-red-600 text-white hover:bg-red-500 active:bg-red-600",
  "outline/accent":
    "border border-[var(--color-border)] text-[var(--color-muted)] hover:border-[var(--color-accent)] hover:text-[var(--color-accent)]",
  "outline/zinc":
    "border border-[var(--color-border)] text-[var(--color-muted)] hover:border-[var(--color-accent)] hover:text-[var(--color-accent)]",
  "outline/red":
    "border border-red-900 text-red-500 hover:border-red-600 hover:text-red-400",
  "plain/accent":
    "text-[var(--color-accent)] hover:opacity-80",
  "plain/zinc":
    "text-[var(--color-muted)] hover:text-[var(--color-text)]",
  "plain/red":
    "text-red-500 hover:text-red-400",
};

export function Button({
  color = "accent",
  outline = false,
  plain = false,
  className,
  children,
  type = "button",
  ...props
}: ButtonProps) {
  const variant = plain ? "plain" : outline ? "outline" : "solid";
  const key = `${variant}/${color}`;
  return (
    <button
      type={type}
      className={clsx(base, styles[key] ?? styles["outline/zinc"], className)}
      {...props}
    >
      {children}
    </button>
  );
}

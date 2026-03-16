import clsx from "clsx";
import { type ComponentPropsWithoutRef } from "react";

const base =
  "rounded border border-[var(--color-border)] bg-[var(--color-canvas)] px-2.5 py-1.5 text-xs " +
  "text-[var(--color-text)] transition-colors cursor-pointer " +
  "focus:border-[var(--color-accent)] focus:outline-none " +
  "disabled:cursor-not-allowed disabled:opacity-40";

export function Select({ className, children, ...props }: ComponentPropsWithoutRef<"select">) {
  return (
    <select className={clsx(base, className)} {...props}>
      {children}
    </select>
  );
}

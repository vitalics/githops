import clsx from "clsx";
import { type ComponentPropsWithoutRef } from "react";

const base =
  "w-full rounded border border-[var(--color-border)] bg-[var(--color-canvas)] px-2.5 py-1.5 text-xs " +
  "text-[var(--color-text)] placeholder:text-[var(--color-muted)] " +
  "transition-colors focus:border-[var(--color-accent)] focus:outline-none " +
  "disabled:cursor-not-allowed disabled:opacity-40";

type InputProps = { invalid?: boolean } & ComponentPropsWithoutRef<"input">;

export function Input({ invalid, className, ...props }: InputProps) {
  return (
    <input
      className={clsx(base, invalid && "border-red-500 focus:border-red-500", className)}
      {...props}
    />
  );
}

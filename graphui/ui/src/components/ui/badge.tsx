import clsx from "clsx";
import { type ComponentPropsWithoutRef } from "react";

type Color = "accent" | "red" | "yellow" | "gray" | "blue";

const colors: Record<Color, string> = {
  accent: "bg-[var(--color-accent-dim)] text-[var(--color-accent)]",
  red: "bg-red-950/40 text-red-400",
  yellow: "bg-yellow-950/40 text-yellow-400",
  gray: "bg-[var(--color-surface)] text-[var(--color-muted)]",
  blue: "bg-blue-950/40 text-blue-400",
};

type BadgeProps = ComponentPropsWithoutRef<"span"> & { color?: Color };

export function Badge({ color = "gray", className, ...props }: BadgeProps) {
  return (
    <span
      className={clsx(
        "inline-flex items-center px-1.5 py-0.5 rounded text-[9px] font-bold",
        colors[color],
        className,
      )}
      {...props}
    />
  );
}

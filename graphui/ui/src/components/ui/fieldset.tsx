import clsx from "clsx";
import { type ComponentPropsWithoutRef } from "react";

export function Fieldset({ className, ...props }: ComponentPropsWithoutRef<"fieldset">) {
  return <fieldset className={clsx("space-y-4", className)} {...props} />;
}

export function Legend({ className, ...props }: ComponentPropsWithoutRef<"legend">) {
  return (
    <legend
      className={clsx("text-xs font-semibold text-[var(--color-text)]", className)}
      {...props}
    />
  );
}

export function FieldGroup({ className, ...props }: ComponentPropsWithoutRef<"div">) {
  return <div className={clsx("space-y-3", className)} {...props} />;
}

export function Field({ className, ...props }: ComponentPropsWithoutRef<"div">) {
  return <div className={clsx("flex flex-col gap-1", className)} {...props} />;
}

export function Label({ className, ...props }: ComponentPropsWithoutRef<"label">) {
  return (
    <label
      className={clsx(
        "text-[9px] font-medium uppercase tracking-widest text-[var(--color-muted)]",
        className,
      )}
      {...props}
    />
  );
}

export function Description({ className, ...props }: ComponentPropsWithoutRef<"p">) {
  return <p className={clsx("text-xs text-[var(--color-muted)]", className)} {...props} />;
}

export function ErrorMessage({ className, ...props }: ComponentPropsWithoutRef<"p">) {
  return <p className={clsx("text-xs text-red-400", className)} {...props} />;
}

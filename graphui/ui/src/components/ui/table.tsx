import clsx from "clsx";
import { type ComponentPropsWithoutRef } from "react";

export function Table({ className, ...props }: ComponentPropsWithoutRef<"table">) {
  return (
    <div className="w-full overflow-hidden rounded-lg border border-[var(--color-border)]">
      <table className={clsx("w-full text-xs", className)} {...props} />
    </div>
  );
}

export function TableHead({ className, ...props }: ComponentPropsWithoutRef<"thead">) {
  return (
    <thead
      className={clsx("border-b border-[var(--color-border)] bg-[var(--color-surface)]", className)}
      {...props}
    />
  );
}

export function TableBody({ className, ...props }: ComponentPropsWithoutRef<"tbody">) {
  return <tbody className={clsx("", className)} {...props} />;
}

export function TableRow({ className, ...props }: ComponentPropsWithoutRef<"tr">) {
  return (
    <tr
      className={clsx(
        "border-b border-[var(--color-border)] last:border-0 hover:bg-[var(--color-surface)]",
        className,
      )}
      {...props}
    />
  );
}

export function TableHeader({ className, ...props }: ComponentPropsWithoutRef<"th">) {
  return (
    <th
      className={clsx(
        "px-3 py-2 text-left text-[9px] uppercase tracking-widest text-[var(--color-muted)] font-normal",
        className,
      )}
      {...props}
    />
  );
}

export function TableCell({ className, ...props }: ComponentPropsWithoutRef<"td">) {
  return <td className={clsx("px-3 py-2 text-[var(--color-text)]", className)} {...props} />;
}

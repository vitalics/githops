import { Checkbox as HLCheckbox } from "@headlessui/react";
import clsx from "clsx";

type CheckboxProps = {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  className?: string;
};

export function Checkbox({ checked, onChange, disabled, className }: CheckboxProps) {
  return (
    <HLCheckbox
      checked={checked}
      onChange={onChange}
      disabled={disabled}
      className={clsx(
        "group relative inline-flex h-3.5 w-3.5 shrink-0 cursor-pointer items-center justify-center rounded",
        "border border-[var(--color-border)] bg-[var(--color-canvas)]",
        "transition-colors focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-accent)]",
        "data-[checked]:border-[var(--color-accent)] data-[checked]:bg-[var(--color-accent)]",
        "data-[disabled]:cursor-not-allowed data-[disabled]:opacity-40",
        className,
      )}
    >
      <svg
        className="pointer-events-none size-2.5 stroke-[#080818] opacity-0 group-data-[checked]:opacity-100"
        viewBox="0 0 14 14"
        fill="none"
      >
        <path d="M3 8L6 11L11 3.5" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" />
      </svg>
    </HLCheckbox>
  );
}

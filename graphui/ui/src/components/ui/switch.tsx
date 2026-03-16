import { Switch as HLSwitch } from "@headlessui/react";
import clsx from "clsx";

type SwitchProps = {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  className?: string;
};

export function Switch({ checked, onChange, disabled, className }: SwitchProps) {
  return (
    <HLSwitch
      checked={checked}
      onChange={onChange}
      disabled={disabled}
      className={clsx(
        "relative inline-flex h-4 w-7 shrink-0 cursor-pointer rounded-full border-2 border-transparent",
        "bg-[var(--color-border)] transition-colors",
        "data-[checked]:bg-[var(--color-accent)]",
        "focus:outline-none focus-visible:ring-2 focus-visible:ring-[var(--color-accent)] focus-visible:ring-offset-1 focus-visible:ring-offset-[var(--color-canvas)]",
        "data-[disabled]:cursor-not-allowed data-[disabled]:opacity-40",
        className,
      )}
    >
      <span
        aria-hidden="true"
        className={clsx(
          "inline-block h-3 w-3 transform rounded-full bg-white shadow-sm transition-transform duration-200 ease-in-out",
          checked ? "translate-x-3" : "translate-x-0",
        )}
      />
    </HLSwitch>
  );
}

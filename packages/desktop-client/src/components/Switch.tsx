import { cn } from "cnfast";
import type { ReactNode } from "react";

export type SwitchOption<T extends string> = {
  disabled?: boolean;
  label: string;
  value: T;
};

type SwitchProps<T extends string> = {
  disabled?: boolean;
  label: string;
  options: SwitchOption<T>[];
  value: T;
  onChange: (value: T) => void;
};

export function Switch<T extends string>({
  disabled = false,
  label,
  options,
  value,
  onChange,
}: SwitchProps<T>): ReactNode {
  return (
    <div className="inline-grid grid-flow-col rounded-lg bg-white/5 p-1 ring-1 ring-white/10">
      {options.map((option) => {
        const isSelected = option.value === value;
        const isDisabled = disabled || option.disabled;

        return (
          <button
            key={option.value}
            type="button"
            aria-label={`${label}: ${option.label}`}
            aria-pressed={isSelected}
            disabled={isDisabled}
            onClick={() => onChange(option.value)}
            className={cn(
              "rounded-md px-2.5 py-1 text-sm/5 font-medium transition disabled:text-zinc-600 disabled:opacity-100 disabled:hover:text-zinc-600",
              isSelected ? "bg-white text-zinc-950 shadow-sm" : "text-zinc-400 hover:text-white"
            )}
          >
            {option.label}
          </button>
        );
      })}
    </div>
  );
}

import {
  Checkbox as HeadlessCheckbox,
  type CheckboxProps as HeadlessCheckboxProps,
  Field as HeadlessField,
  type FieldProps as HeadlessFieldProps,
} from "@headlessui/react";
import { cn } from "cn";
import type React from "react";

export function CheckboxGroup({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return (
    <div
      data-slot="control"
      {...props}
      className={cn(
        className,
        // Basic groups
        "space-y-3",
        // With descriptions
        "has-data-[slot=description]:space-y-6 has-data-[slot=description]:**:data-[slot=label]:font-medium"
      )}
    />
  );
}

export function CheckboxField({
  className,
  ...props
}: { className?: string } & Omit<HeadlessFieldProps, "as" | "className">) {
  return (
    <HeadlessField
      data-slot="field"
      {...props}
      className={cn(
        className,
        // Base layout
        "grid grid-cols-[1.125rem_1fr] gap-x-4 gap-y-1 sm:grid-cols-[1rem_1fr]",
        // Control layout
        "*:data-[slot=control]:col-start-1 *:data-[slot=control]:row-start-1 *:data-[slot=control]:mt-0.75 sm:*:data-[slot=control]:mt-1",
        // Label layout
        "*:data-[slot=label]:col-start-2 *:data-[slot=label]:row-start-1",
        // Description layout
        "*:data-[slot=description]:col-start-2 *:data-[slot=description]:row-start-2",
        // With description
        "has-data-[slot=description]:**:data-[slot=label]:font-medium"
      )}
    />
  );
}

const base = [
  // Basic layout
  "relative isolate flex size-4.5 items-center justify-center rounded-[0.3125rem] sm:size-4",
  // Background color
  "bg-white/5 group-data-checked:bg-(--checkbox-checked-bg)",
  // Border
  "border",
  "border-white/15 group-data-checked:border-white/5 group-data-hover:group-data-checked:border-white/5 group-data-hover:border-white/30",
  // Inner highlight shadow
  "after:absolute after:shadow-[inset_0_1px_--theme(--color-white/15%)]",
  "after:-inset-px after:hidden after:rounded-[0.3125rem] group-data-checked:after:block",
  // Focus ring
  "group-data-focus:outline-2 group-data-focus:outline-offset-2 group-data-focus:outline-blue-500",
  // Disabled state
  "group-data-disabled:opacity-50",
  "group-data-disabled:border-white/20 group-data-disabled:bg-white/2.5 group-data-disabled:[--checkbox-check:var(--color-white)]/50 group-data-checked:group-data-disabled:after:hidden",
  "forced-colors:[--checkbox-check:HighlightText] forced-colors:[--checkbox-checked-bg:Highlight] forced-colors:group-data-disabled:[--checkbox-check:Highlight]",
];

const colors = {
  "dark/zinc": [
    "[--checkbox-check:var(--color-white)]",
    "[--checkbox-checked-bg:var(--color-zinc-600)]",
  ],
  "dark/white": [
    "[--checkbox-check:var(--color-zinc-900)] [--checkbox-checked-bg:var(--color-white)]",
  ],
  white: "[--checkbox-check:var(--color-zinc-900)] [--checkbox-checked-bg:var(--color-white)]",
  dark: "[--checkbox-check:var(--color-white)] [--checkbox-checked-bg:var(--color-zinc-900)]",
  zinc: "[--checkbox-check:var(--color-white)] [--checkbox-checked-bg:var(--color-zinc-600)]",
  red: "[--checkbox-check:var(--color-white)] [--checkbox-checked-bg:var(--color-red-600)]",
  orange: "[--checkbox-check:var(--color-white)] [--checkbox-checked-bg:var(--color-orange-500)]",
  amber: "[--checkbox-check:var(--color-amber-950)] [--checkbox-checked-bg:var(--color-amber-400)]",
  yellow:
    "[--checkbox-check:var(--color-yellow-950)] [--checkbox-checked-bg:var(--color-yellow-300)]",
  lime: "[--checkbox-check:var(--color-lime-950)] [--checkbox-checked-bg:var(--color-lime-300)]",
  green: "[--checkbox-check:var(--color-white)] [--checkbox-checked-bg:var(--color-green-600)]",
  emerald: "[--checkbox-check:var(--color-white)] [--checkbox-checked-bg:var(--color-emerald-600)]",
  teal: "[--checkbox-check:var(--color-white)] [--checkbox-checked-bg:var(--color-teal-600)]",
  cyan: "[--checkbox-check:var(--color-cyan-950)] [--checkbox-checked-bg:var(--color-cyan-300)]",
  sky: "[--checkbox-check:var(--color-white)] [--checkbox-checked-bg:var(--color-sky-500)]",
  blue: "[--checkbox-check:var(--color-white)] [--checkbox-checked-bg:var(--color-blue-600)]",
  indigo: "[--checkbox-check:var(--color-white)] [--checkbox-checked-bg:var(--color-indigo-500)]",
  violet: "[--checkbox-check:var(--color-white)] [--checkbox-checked-bg:var(--color-violet-500)]",
  purple: "[--checkbox-check:var(--color-white)] [--checkbox-checked-bg:var(--color-purple-500)]",
  fuchsia: "[--checkbox-check:var(--color-white)] [--checkbox-checked-bg:var(--color-fuchsia-500)]",
  pink: "[--checkbox-check:var(--color-white)] [--checkbox-checked-bg:var(--color-pink-500)]",
  rose: "[--checkbox-check:var(--color-white)] [--checkbox-checked-bg:var(--color-rose-500)]",
};

type Color = keyof typeof colors;

export function Checkbox({
  color = "dark/zinc",
  className,
  ...props
}: {
  color?: Color;
  className?: string;
} & Omit<HeadlessCheckboxProps, "as" | "className">) {
  return (
    <HeadlessCheckbox
      data-slot="control"
      {...props}
      className={cn(className, "group inline-flex focus:outline-hidden")}
    >
      <span className={cn([base, colors[color]])}>
        <svg
          className="size-4 stroke-(--checkbox-check) opacity-0 group-data-checked:opacity-100 sm:h-3.5 sm:w-3.5"
          fill="none"
          viewBox="0 0 14 14"
        >
          {/* Checkmark icon */}
          <path
            className="opacity-100 group-data-indeterminate:opacity-0"
            d="M3 8L6 11L11 3.5"
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
          />
          {/* Indeterminate icon */}
          <path
            className="opacity-0 group-data-indeterminate:opacity-100"
            d="M3 7H11"
            strokeLinecap="round"
            strokeLinejoin="round"
            strokeWidth={2}
          />
        </svg>
      </span>
    </HeadlessCheckbox>
  );
}

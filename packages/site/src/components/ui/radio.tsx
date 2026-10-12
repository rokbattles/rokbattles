import {
  Field as HeadlessField,
  type FieldProps as HeadlessFieldProps,
  Radio as HeadlessRadio,
  RadioGroup as HeadlessRadioGroup,
  type RadioGroupProps as HeadlessRadioGroupProps,
  type RadioProps as HeadlessRadioProps,
} from "@headlessui/react";
import { cn } from "cn";

export function RadioGroup({
  className,
  ...props
}: { className?: string } & Omit<HeadlessRadioGroupProps, "as" | "className">) {
  return (
    <HeadlessRadioGroup
      data-slot="control"
      {...props}
      className={cn(
        className,
        // Basic groups
        "space-y-3 **:data-[slot=label]:font-normal",
        // With descriptions
        "has-data-[slot=description]:space-y-6 has-data-[slot=description]:**:data-[slot=label]:font-medium"
      )}
    />
  );
}

export function RadioField({
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
  "relative isolate flex size-4.75 shrink-0 rounded-full sm:size-4.25",
  // Background color
  "bg-white/5 group-data-checked:bg-(--radio-checked-bg)",
  // Border
  "border",
  "border-white/15 group-data-checked:border-white/5 group-data-hover:group-data-checked:border-white/5 group-data-hover:border-white/30",
  // Inner highlight shadow
  "after:absolute after:shadow-[inset_0_1px_--theme(--color-white/15%)]",
  "after:-inset-px after:hidden after:rounded-full group-data-checked:after:block",
  // Indicator color
  "[--radio-indicator:transparent] group-data-checked:[--radio-indicator:var(--radio-checked-indicator)]",
  // Hover indicator color
  "group-data-hover:group-data-checked:[--radio-indicator:var(--radio-checked-indicator)] group-data-hover:[--radio-indicator:var(--color-zinc-700)]",
  // Focus ring
  "group-data-focus:outline group-data-focus:outline-2 group-data-focus:outline-offset-2 group-data-focus:outline-blue-500",
  // Disabled state
  "group-data-disabled:opacity-50",
  "group-data-disabled:border-white/20 group-data-disabled:bg-white/2.5 group-data-disabled:[--radio-checked-indicator:var(--color-white)]/50 group-data-checked:group-data-disabled:after:hidden",
];

const colors = {
  "dark/zinc": [
    "[--radio-checked-indicator:var(--color-white)]",
    "[--radio-checked-bg:var(--color-zinc-600)]",
  ],
  "dark/white": [
    "[--radio-checked-bg:var(--color-white)] [--radio-checked-indicator:var(--color-zinc-900)]",
  ],
  white:
    "[--radio-checked-bg:var(--color-white)] [--radio-checked-indicator:var(--color-zinc-900)]",
  dark: "[--radio-checked-bg:var(--color-zinc-900)] [--radio-checked-indicator:var(--color-white)]",
  zinc: "[--radio-checked-indicator:var(--color-white)] [--radio-checked-bg:var(--color-zinc-600)]",
  red: "[--radio-checked-indicator:var(--color-white)] [--radio-checked-bg:var(--color-red-600)]",
  orange:
    "[--radio-checked-indicator:var(--color-white)] [--radio-checked-bg:var(--color-orange-500)]",
  amber:
    "[--radio-checked-bg:var(--color-amber-400)] [--radio-checked-indicator:var(--color-amber-950)]",
  yellow:
    "[--radio-checked-bg:var(--color-yellow-300)] [--radio-checked-indicator:var(--color-yellow-950)]",
  lime: "[--radio-checked-bg:var(--color-lime-300)] [--radio-checked-indicator:var(--color-lime-950)]",
  green:
    "[--radio-checked-indicator:var(--color-white)] [--radio-checked-bg:var(--color-green-600)]",
  emerald:
    "[--radio-checked-indicator:var(--color-white)] [--radio-checked-bg:var(--color-emerald-600)]",
  teal: "[--radio-checked-indicator:var(--color-white)] [--radio-checked-bg:var(--color-teal-600)]",
  cyan: "[--radio-checked-bg:var(--color-cyan-300)] [--radio-checked-indicator:var(--color-cyan-950)]",
  sky: "[--radio-checked-indicator:var(--color-white)] [--radio-checked-bg:var(--color-sky-500)]",
  blue: "[--radio-checked-indicator:var(--color-white)] [--radio-checked-bg:var(--color-blue-600)]",
  indigo:
    "[--radio-checked-indicator:var(--color-white)] [--radio-checked-bg:var(--color-indigo-500)]",
  violet:
    "[--radio-checked-indicator:var(--color-white)] [--radio-checked-bg:var(--color-violet-500)]",
  purple:
    "[--radio-checked-indicator:var(--color-white)] [--radio-checked-bg:var(--color-purple-500)]",
  fuchsia:
    "[--radio-checked-indicator:var(--color-white)] [--radio-checked-bg:var(--color-fuchsia-500)]",
  pink: "[--radio-checked-indicator:var(--color-white)] [--radio-checked-bg:var(--color-pink-500)]",
  rose: "[--radio-checked-indicator:var(--color-white)] [--radio-checked-bg:var(--color-rose-500)]",
};

type Color = keyof typeof colors;

export function Radio({
  color = "dark/zinc",
  className,
  ...props
}: { color?: Color; className?: string } & Omit<
  HeadlessRadioProps,
  "as" | "className" | "children"
>) {
  return (
    <HeadlessRadio
      data-slot="control"
      {...props}
      className={cn(className, "group inline-flex focus:outline-hidden")}
    >
      <span className={cn([base, colors[color]])}>
        <span
          className={cn(
            "size-full rounded-full border-[4.5px] border-transparent bg-(--radio-indicator) bg-clip-padding",
            // Forced colors mode
            "forced-colors:border-[Canvas] forced-colors:group-data-checked:border-[Highlight]"
          )}
        />
      </span>
    </HeadlessRadio>
  );
}

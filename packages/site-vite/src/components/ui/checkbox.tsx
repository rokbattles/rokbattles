import { Checkbox as BaseCheckbox } from "@base-ui/react/checkbox";
import { Field as BaseField } from "@base-ui/react/field";
import { cn } from "cn";
import { Check, Minus } from "lucide-react";
import type React from "react";

export function CheckboxGroup({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return (
    <div
      data-slot="control"
      {...props}
      className={cn(
        className,
        "space-y-3 has-data-[slot=description]:space-y-6 has-data-[slot=description]:**:data-[slot=label]:font-medium"
      )}
    />
  );
}

export function CheckboxField({
  className,
  ...props
}: { className?: string } & Omit<BaseField.Root.Props, "className">) {
  return (
    <BaseField.Root
      data-slot="field"
      {...props}
      className={cn(
        className,
        "grid grid-cols-[1.125rem_1fr] gap-x-4 gap-y-1 sm:grid-cols-[1rem_1fr] *:data-[slot=control]:col-start-1 *:data-[slot=control]:row-start-1 *:data-[slot=control]:mt-0.75 sm:*:data-[slot=control]:mt-1 *:data-[slot=label]:col-start-2 *:data-[slot=label]:row-start-1 *:data-[slot=description]:col-start-2 *:data-[slot=description]:row-start-2 has-data-[slot=description]:**:data-[slot=label]:font-medium"
      )}
    />
  );
}

const base =
  "relative isolate flex size-4.5 items-center justify-center rounded-[0.3125rem] sm:size-4 border after:absolute after:shadow-[inset_0_1px_--theme(--color-white/15%)] group-focus-visible:outline-2 group-focus-visible:outline-offset-2 group-focus-visible:outline-blue-500 group-data-disabled:opacity-50 bg-white/5 group-data-checked:bg-(--checkbox-checked-bg) group-data-indeterminate:bg-(--checkbox-checked-bg) border-white/15 group-data-checked:border-white/5 group-data-indeterminate:border-white/5 group-hover:group-data-checked:border-white/5 group-hover:group-data-indeterminate:border-white/5 group-hover:border-white/30 after:-inset-px after:hidden after:rounded-[0.3125rem] group-data-checked:after:block group-data-indeterminate:after:block group-data-disabled:border-white/20 group-data-disabled:bg-white/2.5 group-data-disabled:[--checkbox-check:var(--color-white)]/50 group-data-checked:group-data-disabled:after:hidden group-data-indeterminate:group-data-disabled:after:hidden forced-colors:[--checkbox-check:HighlightText] forced-colors:[--checkbox-checked-bg:Highlight] forced-colors:group-data-disabled:[--checkbox-check:Highlight]";

const colors = {
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
  color = "zinc",
  className,
  ...props
}: {
  color?: Color;
  className?: string;
} & Omit<BaseCheckbox.Root.Props, "className">) {
  return (
    <BaseCheckbox.Root
      data-slot="control"
      {...props}
      className={cn(className, "group inline-flex focus:outline-hidden")}
    >
      <span className={cn([base, colors[color]])}>
        <BaseCheckbox.Indicator
          keepMounted
          className="relative size-4 text-(--checkbox-check) sm:size-3.5"
        >
          <Check
            aria-hidden="true"
            className="size-full group-data-unchecked:hidden group-data-indeterminate:hidden"
          />
          <Minus
            aria-hidden="true"
            className="absolute inset-0 hidden size-full group-data-indeterminate:block"
          />
        </BaseCheckbox.Indicator>
      </span>
    </BaseCheckbox.Root>
  );
}

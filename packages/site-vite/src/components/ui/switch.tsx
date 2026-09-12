import { Field as BaseField } from "@base-ui/react/field";
import { Switch as BaseSwitch } from "@base-ui/react/switch";
import clsx from "clsx";
import type React from "react";

export function SwitchGroup({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return (
    <div
      data-slot="control"
      {...props}
      className={clsx(
        className,
        "space-y-3 **:data-[slot=label]:font-normal has-data-[slot=description]:space-y-6 has-data-[slot=description]:**:data-[slot=label]:font-medium"
      )}
    />
  );
}

export function SwitchField({
  className,
  ...props
}: { className?: string } & Omit<BaseField.Root.Props, "className">) {
  return (
    <BaseField.Root
      data-slot="field"
      {...props}
      className={clsx(
        className,
        "grid grid-cols-[1fr_auto] gap-x-8 gap-y-1 sm:grid-cols-[1fr_auto] *:data-[slot=control]:col-start-2 *:data-[slot=control]:self-start sm:*:data-[slot=control]:mt-0.5 *:data-[slot=label]:col-start-1 *:data-[slot=label]:row-start-1 *:data-[slot=description]:col-start-1 *:data-[slot=description]:row-start-2 has-data-[slot=description]:**:data-[slot=label]:font-medium"
      )}
    />
  );
}

const colors = {
  neutral:
    "[--switch-bg:var(--color-white)]/25 [--switch-bg-ring:transparent] [--switch-ring:var(--color-zinc-700)]/90 [--switch-shadow:var(--color-black)]/10 [--switch:white]",
  dark: "[--switch-bg:var(--color-zinc-900)] [--switch-ring:var(--color-zinc-950)]/90 [--switch-shadow:var(--color-black)]/10 [--switch:white] [--switch-bg-ring:var(--color-white)]/15",
  zinc: "[--switch-bg:var(--color-zinc-600)] [--switch-shadow:var(--color-black)]/10 [--switch:white] [--switch-ring:var(--color-zinc-700)]/90 [--switch-bg-ring:transparent]",
  white:
    "[--switch-bg:white] [--switch-shadow:var(--color-black)]/10 [--switch-ring:transparent] [--switch:var(--color-zinc-950)] [--switch-bg-ring:transparent]",
  red: "[--switch-bg:var(--color-red-600)] [--switch:white] [--switch-ring:var(--color-red-700)]/90 [--switch-shadow:var(--color-red-900)]/20 [--switch-bg-ring:transparent]",
  orange:
    "[--switch-bg:var(--color-orange-500)] [--switch:white] [--switch-ring:var(--color-orange-600)]/90 [--switch-shadow:var(--color-orange-900)]/20 [--switch-bg-ring:transparent]",
  amber:
    "[--switch-bg:var(--color-amber-400)] [--switch-ring:transparent] [--switch-shadow:transparent] [--switch:var(--color-amber-950)] [--switch-bg-ring:transparent]",
  yellow:
    "[--switch-bg:var(--color-yellow-300)] [--switch-ring:transparent] [--switch-shadow:transparent] [--switch:var(--color-yellow-950)] [--switch-bg-ring:transparent]",
  lime: "[--switch-bg:var(--color-lime-300)] [--switch-ring:transparent] [--switch-shadow:transparent] [--switch:var(--color-lime-950)] [--switch-bg-ring:transparent]",
  green:
    "[--switch-bg:var(--color-green-600)] [--switch:white] [--switch-ring:var(--color-green-700)]/90 [--switch-shadow:var(--color-green-900)]/20 [--switch-bg-ring:transparent]",
  emerald:
    "[--switch-bg:var(--color-emerald-500)] [--switch:white] [--switch-ring:var(--color-emerald-600)]/90 [--switch-shadow:var(--color-emerald-900)]/20 [--switch-bg-ring:transparent]",
  teal: "[--switch-bg:var(--color-teal-600)] [--switch:white] [--switch-ring:var(--color-teal-700)]/90 [--switch-shadow:var(--color-teal-900)]/20 [--switch-bg-ring:transparent]",
  cyan: "[--switch-bg:var(--color-cyan-300)] [--switch-ring:transparent] [--switch-shadow:transparent] [--switch:var(--color-cyan-950)] [--switch-bg-ring:transparent]",
  sky: "[--switch-bg:var(--color-sky-500)] [--switch:white] [--switch-ring:var(--color-sky-600)]/80 [--switch-shadow:var(--color-sky-900)]/20 [--switch-bg-ring:transparent]",
  blue: "[--switch-bg:var(--color-blue-600)] [--switch:white] [--switch-ring:var(--color-blue-700)]/90 [--switch-shadow:var(--color-blue-900)]/20 [--switch-bg-ring:transparent]",
  indigo:
    "[--switch-bg:var(--color-indigo-500)] [--switch:white] [--switch-ring:var(--color-indigo-600)]/90 [--switch-shadow:var(--color-indigo-900)]/20 [--switch-bg-ring:transparent]",
  violet:
    "[--switch-bg:var(--color-violet-500)] [--switch:white] [--switch-ring:var(--color-violet-600)]/90 [--switch-shadow:var(--color-violet-900)]/20 [--switch-bg-ring:transparent]",
  purple:
    "[--switch-bg:var(--color-purple-500)] [--switch:white] [--switch-ring:var(--color-purple-600)]/90 [--switch-shadow:var(--color-purple-900)]/20 [--switch-bg-ring:transparent]",
  fuchsia:
    "[--switch-bg:var(--color-fuchsia-500)] [--switch:white] [--switch-ring:var(--color-fuchsia-600)]/90 [--switch-shadow:var(--color-fuchsia-900)]/20 [--switch-bg-ring:transparent]",
  pink: "[--switch-bg:var(--color-pink-500)] [--switch:white] [--switch-ring:var(--color-pink-600)]/90 [--switch-shadow:var(--color-pink-900)]/20 [--switch-bg-ring:transparent]",
  rose: "[--switch-bg:var(--color-rose-500)] [--switch:white] [--switch-ring:var(--color-rose-600)]/90 [--switch-shadow:var(--color-rose-900)]/20 [--switch-bg-ring:transparent]",
};

type Color = keyof typeof colors;

export function Switch({
  color = "neutral",
  className,
  ...props
}: {
  color?: Color;
  className?: string;
} & Omit<BaseSwitch.Root.Props, "className" | "children">) {
  return (
    <BaseSwitch.Root
      data-slot="control"
      {...props}
      className={clsx(
        className,
        "group relative isolate inline-flex h-6 w-10 cursor-default rounded-full p-[3px] sm:h-5 sm:w-8 transition-colors duration-200 ease-in-out motion-reduce:transition-none forced-colors:outline ring-1 ring-inset focus:not-focus-visible:outline-hidden focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500 data-disabled:opacity-50 forced-colors:[--switch-bg:Highlight] bg-white/5 ring-white/15 data-checked:bg-(--switch-bg) data-checked:ring-(--switch-bg-ring) hover:ring-white/25 hover:data-checked:ring-(--switch-bg-ring) data-disabled:bg-white/15 data-disabled:data-checked:bg-white/15 data-disabled:data-checked:ring-white/15",
        colors[color]
      )}
    >
      <BaseSwitch.Thumb
        aria-hidden="true"
        className={
          "pointer-events-none relative inline-block size-4.5 rounded-full sm:size-3.5 translate-x-0 transition duration-200 ease-in-out motion-reduce:transition-none border border-transparent bg-white shadow-sm ring-1 ring-black/5 group-data-checked:bg-(--switch) group-data-checked:shadow-(--switch-shadow) group-data-checked:ring-(--switch-ring) group-data-checked:translate-x-4 sm:group-data-checked:translate-x-3 group-data-checked:group-data-disabled:bg-white group-data-checked:group-data-disabled:shadow-sm group-data-checked:group-data-disabled:ring-black/5"
        }
      />
    </BaseSwitch.Root>
  );
}

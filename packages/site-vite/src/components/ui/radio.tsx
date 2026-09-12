import { Field as BaseField } from "@base-ui/react/field";
import { Radio as BaseRadio } from "@base-ui/react/radio";
import { RadioGroup as BaseRadioGroup } from "@base-ui/react/radio-group";
import clsx from "clsx";

export function RadioGroup<Value>({
  className,
  ...props
}: { className?: string } & Omit<BaseRadioGroup.Props<Value>, "className">) {
  return (
    <BaseRadioGroup
      data-slot="control"
      {...props}
      className={clsx(
        className,
        "space-y-3 **:data-[slot=label]:font-normal has-data-[slot=description]:space-y-6 has-data-[slot=description]:**:data-[slot=label]:font-medium"
      )}
    />
  );
}

export function RadioField({
  className,
  ...props
}: { className?: string } & Omit<BaseField.Item.Props, "className">) {
  return (
    <BaseField.Item
      data-slot="field"
      {...props}
      className={clsx(
        className,
        "grid grid-cols-[1.125rem_1fr] gap-x-4 gap-y-1 sm:grid-cols-[1rem_1fr] *:data-[slot=control]:col-start-1 *:data-[slot=control]:row-start-1 *:data-[slot=control]:mt-0.75 sm:*:data-[slot=control]:mt-1 *:data-[slot=label]:col-start-2 *:data-[slot=label]:row-start-1 *:data-[slot=description]:col-start-2 *:data-[slot=description]:row-start-2 has-data-[slot=description]:**:data-[slot=label]:font-medium"
      )}
    />
  );
}

const base =
  "relative isolate flex size-4.75 shrink-0 rounded-full sm:size-4.25 border after:absolute after:shadow-[inset_0_1px_--theme(--color-white/15%)] [--radio-indicator:transparent] group-data-checked:[--radio-indicator:var(--radio-checked-indicator)] group-focus-visible:outline-2 group-focus-visible:outline-offset-2 group-focus-visible:outline-blue-500 group-data-disabled:opacity-50 bg-white/5 group-data-checked:bg-(--radio-checked-bg) border-white/15 group-data-checked:border-white/5 group-hover:group-data-checked:border-white/5 group-hover:border-white/30 after:-inset-px after:hidden after:rounded-full group-data-checked:after:block group-hover:group-data-checked:[--radio-indicator:var(--radio-checked-indicator)] group-hover:[--radio-indicator:var(--color-zinc-700)] group-data-disabled:border-white/20 group-data-disabled:bg-white/2.5 group-data-disabled:[--radio-checked-indicator:var(--color-white)]/50 group-data-checked:group-data-disabled:after:hidden";

const colors = {
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

export function Radio<Value>({
  color = "zinc",
  className,
  ...props
}: { color?: Color; className?: string } & Omit<
  BaseRadio.Root.Props<Value>,
  "className" | "children"
>) {
  return (
    <BaseRadio.Root
      data-slot="control"
      {...props}
      className={clsx(className, "group inline-flex focus:outline-hidden")}
    >
      <span className={clsx([base, colors[color]])}>
        <span
          className={
            "size-full rounded-full border-[4.5px] border-transparent bg-(--radio-indicator) bg-clip-padding forced-colors:border-[Canvas] forced-colors:group-data-checked:border-[Highlight]"
          }
        />
      </span>
    </BaseRadio.Root>
  );
}

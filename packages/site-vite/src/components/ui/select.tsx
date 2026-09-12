import { Field } from "@base-ui/react/field";
import { cn } from "cn";
import { ChevronsUpDown } from "lucide-react";
import type { ComponentProps } from "react";

type SelectProps = ComponentProps<"select"> & {
  onValueChange?: Field.Control.Props["onValueChange"];
};

// Keep the native select API, including <option>, <optgroup>, and multiple.
export function Select({
  className,
  multiple,
  ref,
  name,
  disabled,
  value,
  defaultValue,
  onValueChange,
  ...props
}: SelectProps) {
  return (
    <span
      data-slot="control"
      className={cn(className, "group relative block w-full has-data-disabled:opacity-50")}
    >
      <Field.Control
        ref={ref}
        name={name}
        disabled={disabled}
        value={value}
        defaultValue={defaultValue}
        onValueChange={onValueChange}
        render={<select multiple={multiple} {...props} />}
        className={cn(
          "relative block w-full appearance-none rounded-lg border border-white/10 bg-white/5 py-[calc(--spacing(2.5)-1px)] text-base/6 text-white scheme-dark hover:border-white/20 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500 sm:py-[calc(--spacing(1.5)-1px)] sm:text-sm/6 [&_optgroup]:font-semibold [&_option]:bg-zinc-800 [&_optgroup]:bg-zinc-800",
          "data-invalid:border-red-600 data-invalid:hover:border-red-600 data-disabled:border-white/15 data-disabled:bg-white/2.5 data-disabled:hover:border-white/15",
          multiple
            ? "px-[calc(--spacing(3.5)-1px)] sm:px-[calc(--spacing(3)-1px)]"
            : "pr-[calc(--spacing(10)-1px)] pl-[calc(--spacing(3.5)-1px)] sm:pr-[calc(--spacing(9)-1px)] sm:pl-[calc(--spacing(3)-1px)]"
        )}
      />
      {!multiple && (
        <ChevronsUpDown
          aria-hidden="true"
          className="pointer-events-none absolute top-1/2 right-2 size-5 -translate-y-1/2 text-zinc-400 group-has-data-disabled:text-zinc-600 sm:size-4 forced-colors:text-[CanvasText]"
        />
      )}
    </span>
  );
}

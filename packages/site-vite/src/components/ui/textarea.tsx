import { Field } from "@base-ui/react/field";
import { cn } from "cn";
import type { ComponentProps } from "react";

type TextareaProps = ComponentProps<"textarea"> & {
  resizable?: boolean;
  onValueChange?: Field.Control.Props["onValueChange"];
};

export function Textarea({
  className,
  resizable = true,
  ref,
  name,
  disabled,
  value,
  defaultValue,
  onValueChange,
  ...props
}: TextareaProps) {
  return (
    <span
      data-slot="control"
      className={cn(className, "relative block w-full has-data-disabled:opacity-50")}
    >
      <Field.Control
        ref={ref}
        name={name}
        disabled={disabled}
        value={value}
        defaultValue={defaultValue}
        onValueChange={onValueChange}
        render={<textarea {...props} />}
        className={cn(
          "relative block h-full w-full appearance-none rounded-lg border border-white/10 bg-white/5 px-[calc(--spacing(3.5)-1px)] py-[calc(--spacing(2.5)-1px)] text-base/6 text-white scheme-dark placeholder:text-zinc-500 hover:border-white/20 focus:outline-2 focus:outline-offset-2 focus:outline-blue-500 sm:px-[calc(--spacing(3)-1px)] sm:py-[calc(--spacing(1.5)-1px)] sm:text-sm/6",
          "data-invalid:border-red-600 data-invalid:hover:border-red-600 data-disabled:border-white/15 data-disabled:bg-white/2.5 data-disabled:hover:border-white/15",
          resizable ? "resize-y" : "resize-none"
        )}
      />
    </span>
  );
}

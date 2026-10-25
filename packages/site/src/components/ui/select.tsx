"use client";

import { Field as BaseField } from "@base-ui/react/field";
import { cn } from "cn";
import { ChevronsUpDown } from "lucide-react";
import type React from "react";
import { forwardRef } from "react";

export const Select = forwardRef(function Select(
  {
    className,
    invalid,
    multiple,
    ...props
  }: { invalid?: boolean; className?: string } & Omit<
    React.ComponentPropsWithoutRef<"select">,
    "as" | "className"
  >,
  ref: React.ForwardedRef<HTMLSelectElement>
) {
  return (
    <span
      className={cn([
        className,
        // Basic layout
        "group relative block w-full",
        // Focus ring
        "after:pointer-events-none after:absolute after:inset-0 after:rounded-lg after:ring-transparent after:ring-inset has-[:focus-visible]:after:ring-2 has-[:focus-visible]:after:ring-blue-500",
        // Disabled state
        "has-data-disabled:opacity-50",
      ])}
      data-slot="control"
    >
      <BaseField.Control
        data-autofocus={props.autoFocus || undefined}
        ref={ref}
        disabled={props.disabled}
        value={props.value}
        defaultValue={props.defaultValue}
        aria-invalid={invalid || props["aria-invalid"]}
        data-invalid={invalid || undefined}
        render={<select {...props} multiple={multiple} />}
        className={cn([
          // Basic layout
          "relative block w-full appearance-none rounded-lg py-[calc(--spacing(2.5)-1px)] sm:py-[calc(--spacing(1.5)-1px)]",
          // Horizontal padding
          multiple
            ? "px-[calc(--spacing(3.5)-1px)] sm:px-[calc(--spacing(3)-1px)]"
            : "pr-[calc(--spacing(10)-1px)] pl-[calc(--spacing(3.5)-1px)] sm:pr-[calc(--spacing(9)-1px)] sm:pl-[calc(--spacing(3)-1px)]",
          // Options (multi-select)
          "[&_optgroup]:font-semibold",
          // Typography
          "text-base/6 placeholder:text-zinc-500 sm:text-sm/6 text-white *:text-white",
          // Border
          "border border-white/10 hover:border-white/20",
          // Background color
          "bg-white/5 *:bg-zinc-800",
          // Hide default focus styles
          "focus:outline-hidden",
          // Invalid state
          "data-invalid:hover:border-red-600 data-invalid:border-red-600",
          // Disabled state
          "data-disabled:opacity-100 hover:data-disabled:border-white/15 data-disabled:border-white/15 data-disabled:bg-white/2.5",
        ])}
      />
      {!multiple && (
        <span className="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-2">
          <ChevronsUpDown
            aria-hidden="true"
            className="size-5 group-has-data-disabled:text-zinc-600 sm:size-4 text-zinc-400 forced-colors:text-[CanvasText]"
          />
        </span>
      )}
    </span>
  );
});

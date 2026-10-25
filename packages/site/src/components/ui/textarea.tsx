"use client";

import { Field as BaseField } from "@base-ui/react/field";
import { cn } from "cn";
import type React from "react";
import { forwardRef } from "react";

export const Textarea = forwardRef(function Textarea(
  {
    className,
    invalid,
    resizable = true,
    ...props
  }: { invalid?: boolean; className?: string; resizable?: boolean } & Omit<
    React.ComponentPropsWithoutRef<"textarea">,
    "as" | "className"
  >,
  ref: React.ForwardedRef<HTMLTextAreaElement>
) {
  return (
    <span
      className={cn([
        className,
        // Basic layout
        "relative block w-full",
        // Focus ring
        "after:pointer-events-none after:absolute after:inset-0 after:rounded-lg after:ring-transparent after:ring-inset sm:focus-within:after:ring-2 sm:focus-within:after:ring-blue-500",
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
        render={<textarea {...props} />}
        className={cn([
          // Basic layout
          "relative block h-full w-full appearance-none rounded-lg px-[calc(--spacing(3.5)-1px)] py-[calc(--spacing(2.5)-1px)] sm:px-[calc(--spacing(3)-1px)] sm:py-[calc(--spacing(1.5)-1px)]",
          // Typography
          "text-base/6 placeholder:text-zinc-500 sm:text-sm/6 text-white",
          // Border
          "border border-white/10 hover:border-white/20",
          // Background color
          "bg-white/5",
          // Hide default focus styles
          "focus:outline-hidden",
          // Invalid state
          "data-invalid:hover:border-red-600 data-invalid:border-red-600",
          // Disabled state
          "disabled:border-white/15 disabled:bg-white/2.5 hover:disabled:border-white/15",
          // Resizable
          resizable ? "resize-y" : "resize-none",
        ])}
      />
    </span>
  );
});

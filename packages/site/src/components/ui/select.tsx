import {
  Select as HeadlessSelect,
  type SelectProps as HeadlessSelectProps,
} from "@headlessui/react";
import { cn } from "cn";
import type React from "react";
import { forwardRef } from "react";

export const Select = forwardRef(function Select(
  {
    className,
    multiple,
    ...props
  }: { className?: string } & Omit<HeadlessSelectProps, "as" | "className">,
  ref: React.ForwardedRef<HTMLSelectElement>
) {
  return (
    <span
      className={cn([
        className,
        // Basic layout
        "group relative block w-full",
        // Focus ring
        "after:pointer-events-none after:absolute after:inset-0 after:rounded-lg after:ring-transparent after:ring-inset has-data-focus:after:ring-2 has-data-focus:after:ring-blue-500",
        // Disabled state
        "has-data-disabled:opacity-50",
      ])}
      data-slot="control"
    >
      <HeadlessSelect
        multiple={multiple}
        ref={ref}
        {...props}
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
          "border border-white/10 data-hover:border-white/20",
          // Background color
          "bg-white/5 *:bg-zinc-800",
          // Hide default focus styles
          "focus:outline-hidden",
          // Invalid state
          "data-invalid:data-hover:border-red-600 data-invalid:border-red-600",
          // Disabled state
          "data-disabled:opacity-100 data-hover:data-disabled:border-white/15 data-disabled:border-white/15 data-disabled:bg-white/2.5",
        ])}
      />
      {!multiple && (
        <span className="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-2">
          <svg
            aria-hidden="true"
            className="size-5 group-has-data-disabled:stroke-zinc-600 sm:size-4 stroke-zinc-400 forced-colors:stroke-[CanvasText]"
            fill="none"
            viewBox="0 0 16 16"
          >
            <path
              d="M5.75 10.75L8 13L10.25 10.75"
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={1.5}
            />
            <path
              d="M10.25 5.25L8 3L5.75 5.25"
              strokeLinecap="round"
              strokeLinejoin="round"
              strokeWidth={1.5}
            />
          </svg>
        </span>
      )}
    </span>
  );
});

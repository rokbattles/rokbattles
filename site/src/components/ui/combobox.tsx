"use client";

import {
  Combobox as HeadlessCombobox,
  ComboboxButton as HeadlessComboboxButton,
  ComboboxInput as HeadlessComboboxInput,
  ComboboxOption as HeadlessComboboxOption,
  type ComboboxOptionProps as HeadlessComboboxOptionProps,
  ComboboxOptions as HeadlessComboboxOptions,
  type ComboboxProps as HeadlessComboboxProps,
} from "@headlessui/react";
import type React from "react";
import { useState } from "react";
import { cn } from "@/lib/cn";

export function Combobox<T>({
  options,
  displayValue,
  filter,
  anchor = "bottom",
  className,
  placeholder,
  autoFocus,
  "aria-label": ariaLabel,
  children,
  ...props
}: {
  options: T[];
  displayValue: (value: T | null) => string | undefined;
  filter?: (value: T, query: string) => boolean;
  className?: string;
  placeholder?: string;
  autoFocus?: boolean;
  "aria-label"?: string;
  children: (value: NonNullable<T>) => React.ReactElement;
} & Omit<HeadlessComboboxProps<T, false>, "as" | "multiple" | "children"> & {
    anchor?: "top" | "bottom";
  }) {
  const [query, setQuery] = useState("");

  const filteredOptions =
    query === ""
      ? options
      : options.filter((option) =>
          filter
            ? filter(option, query)
            : displayValue(option)?.toLowerCase().includes(query.toLowerCase())
        );

  return (
    <HeadlessCombobox
      {...props}
      multiple={false}
      onClose={() => setQuery("")}
      virtual={{ options: filteredOptions }}
    >
      <span
        className={cn([
          className,
          // Basic layout
          "relative block w-full",
          // Background color + shadow applied to inset pseudo element, so shadow blends with border in light mode
          "before:absolute before:inset-px before:rounded-[calc(var(--radius-lg)-1px)] before:bg-white before:shadow-sm",
          // Background color is moved to control and shadow is removed in dark mode so hide `before` pseudo
          "dark:before:hidden",
          // Focus ring
          "after:pointer-events-none after:absolute after:inset-0 after:rounded-lg after:ring-transparent after:ring-inset sm:focus-within:after:ring-2 sm:focus-within:after:ring-blue-500",
          // Disabled state
          "has-data-disabled:opacity-50 has-data-disabled:before:bg-zinc-950/5 has-data-disabled:before:shadow-none",
          // Invalid state
          "has-data-invalid:before:shadow-red-500/10",
        ])}
        data-slot="control"
      >
        <HeadlessComboboxInput
          aria-label={ariaLabel}
          autoFocus={autoFocus}
          className={cn([
            className,
            // Basic layout
            "relative block w-full appearance-none rounded-lg py-[calc(--spacing(2.5)-1px)] sm:py-[calc(--spacing(1.5)-1px)]",
            // Horizontal padding
            "pr-[calc(--spacing(10)-1px)] pl-[calc(--spacing(3.5)-1px)] sm:pr-[calc(--spacing(9)-1px)] sm:pl-[calc(--spacing(3)-1px)]",
            // Typography
            "text-base/6 text-zinc-950 placeholder:text-zinc-500 sm:text-sm/6 dark:text-white",
            // Border
            "border border-zinc-950/10 data-hover:border-zinc-950/20 dark:border-white/10 dark:data-hover:border-white/20",
            // Background color
            "bg-transparent dark:bg-white/5",
            // Hide default focus styles
            "focus:outline-hidden",
            // Invalid state
            "data-invalid:data-hover:border-red-500 data-invalid:border-red-500 dark:data-invalid:data-hover:border-red-500 dark:data-invalid:border-red-500",
            // Disabled state
            "data-disabled:border-zinc-950/20 dark:data-hover:data-disabled:border-white/15 dark:data-disabled:border-white/15 dark:data-disabled:bg-white/2.5",
            // System icons
            "dark:scheme-dark",
          ])}
          data-slot="control"
          displayValue={(option: T) => displayValue(option) ?? ""}
          onChange={(event) => setQuery(event.target.value)}
          placeholder={placeholder}
        />
        <HeadlessComboboxButton className="group absolute inset-y-0 right-0 flex items-center px-2">
          <svg
            aria-hidden="true"
            className="size-5 stroke-zinc-500 group-data-disabled:stroke-zinc-600 group-data-hover:stroke-zinc-700 sm:size-4 dark:stroke-zinc-400 dark:group-data-hover:stroke-zinc-300 forced-colors:stroke-[CanvasText]"
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
        </HeadlessComboboxButton>
      </span>
      <HeadlessComboboxOptions
        anchor={anchor}
        className={cn(
          // Anchor positioning
          "[--anchor-gap:--spacing(2)] [--anchor-padding:--spacing(4)] sm:data-[anchor~=start]:[--anchor-offset:-4px]",
          // Base styles,
          "isolate min-w-[calc(var(--input-width)+8px)] select-none scroll-py-1 rounded-xl p-1 empty:invisible",
          // Invisible border that is only visible in `forced-colors` mode for accessibility purposes
          "outline outline-transparent focus:outline-hidden",
          // Handle scrolling when menu won't fit in viewport
          "overflow-y-scroll overscroll-contain",
          // Popover background
          "bg-white/75 backdrop-blur-xl dark:bg-zinc-800/75",
          // Shadows
          "shadow-lg ring-1 ring-zinc-950/10 dark:ring-white/10 dark:ring-inset",
          // Transitions
          "transition-opacity duration-100 ease-in data-closed:data-leave:opacity-0 data-transition:pointer-events-none"
        )}
        transition
      >
        {({ option }) => children(option)}
      </HeadlessComboboxOptions>
    </HeadlessCombobox>
  );
}

export function ComboboxOption<T>({
  children,
  className,
  ...props
}: { className?: string; children?: React.ReactNode } & Omit<
  HeadlessComboboxOptionProps<"div", T>,
  "as" | "className"
>) {
  const sharedClasses = cn(
    // Base
    "flex min-w-0 items-center",
    // Icons
    "*:data-[slot=icon]:size-5 *:data-[slot=icon]:shrink-0 sm:*:data-[slot=icon]:size-4",
    "*:data-[slot=icon]:text-zinc-500 group-data-focus/option:*:data-[slot=icon]:text-white dark:*:data-[slot=icon]:text-zinc-400",
    "forced-colors:*:data-[slot=icon]:text-[CanvasText] forced-colors:group-data-focus/option:*:data-[slot=icon]:text-[Canvas]",
    // Avatars
    "*:data-[slot=avatar]:-mx-0.5 *:data-[slot=avatar]:size-6 sm:*:data-[slot=avatar]:size-5"
  );

  return (
    <HeadlessComboboxOption
      {...props}
      className={cn(
        // Basic layout
        "group/option grid w-full cursor-default grid-cols-[1fr_--spacing(5)] items-baseline gap-x-2 rounded-lg py-2.5 pr-2 pl-3.5 sm:grid-cols-[1fr_--spacing(4)] sm:py-1.5 sm:pr-2 sm:pl-3",
        // Typography
        "text-base/6 text-zinc-950 sm:text-sm/6 dark:text-white forced-colors:text-[CanvasText]",
        // Focus
        "outline-hidden data-focus:bg-blue-500 data-focus:text-white",
        // Forced colors mode
        "forced-color-adjust-none forced-colors:data-focus:bg-[Highlight] forced-colors:data-focus:text-[HighlightText]",
        // Disabled
        "data-disabled:opacity-50"
      )}
    >
      <span className={cn(className, sharedClasses)}>{children}</span>
      <svg
        aria-hidden="true"
        className="relative col-start-2 hidden size-5 self-center stroke-current group-data-selected/option:inline sm:size-4"
        fill="none"
        viewBox="0 0 16 16"
      >
        <path d="M4 8.5l3 3L12 4" strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} />
      </svg>
    </HeadlessComboboxOption>
  );
}

export function ComboboxLabel({ className, ...props }: React.ComponentPropsWithoutRef<"span">) {
  return (
    <span
      {...props}
      className={cn(className, "ml-2.5 truncate first:ml-0 sm:ml-2 sm:first:ml-0")}
    />
  );
}

export function ComboboxDescription({
  className,
  children,
  ...props
}: React.ComponentPropsWithoutRef<"span">) {
  return (
    <span
      {...props}
      className={cn(
        className,
        "flex flex-1 overflow-hidden text-zinc-500 before:w-2 before:min-w-0 before:shrink group-data-focus/option:text-white dark:text-zinc-400"
      )}
    >
      <span className="flex-1 truncate">{children}</span>
    </span>
  );
}

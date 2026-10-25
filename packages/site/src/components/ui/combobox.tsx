"use client";

import { Combobox as BaseCombobox } from "@base-ui/react/combobox";
import { cn } from "cn";
import { Check, ChevronsUpDown } from "lucide-react";
import type React from "react";
import { cloneElement, useMemo, useRef } from "react";

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
  onChange,
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
} & Omit<
  BaseCombobox.Root.Props<T, false>,
  "multiple" | "children" | "onValueChange" | "filter"
> & { onChange?: (value: NoInfer<T> | null) => void } & {
    anchor?: "top" | "bottom";
  }) {
  const anchorRef = useRef<HTMLSpanElement>(null);
  const optionKeys = useMemo(
    () => new Map(options.map((option, index) => [option, index])),
    [options]
  );
  return (
    <BaseCombobox.Root
      {...props}
      items={options}
      itemToStringLabel={(option) => displayValue(option) ?? ""}
      filter={(option, query) =>
        !query ||
        (filter
          ? filter(option, query)
          : (displayValue(option)?.toLowerCase().includes(query.toLowerCase()) ?? false))
      }
      onInputValueChange={(value, details) => {
        props.onInputValueChange?.(value, details);
        // Escape dismisses the popup; it must not clear a closed control's selection.
        if (details.reason === "escape-key") details.cancel();
      }}
      onValueChange={(value, details) => {
        if (details.reason === "escape-key") details.cancel();
        else onChange?.(value);
      }}
      autoHighlight
      loopFocus={false}
      openOnInputClick={false}
    >
      <span
        ref={anchorRef}
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
        <BaseCombobox.Input
          data-autofocus={autoFocus || undefined}
          aria-label={ariaLabel}
          autoFocus={autoFocus}
          className={cn([
            className,
            // Basic layout
            "relative block w-full appearance-none rounded-lg py-[calc(--spacing(2.5)-1px)] sm:py-[calc(--spacing(1.5)-1px)]",
            // Horizontal padding
            "pr-[calc(--spacing(10)-1px)] pl-[calc(--spacing(3.5)-1px)] sm:pr-[calc(--spacing(9)-1px)] sm:pl-[calc(--spacing(3)-1px)]",
            // Typography
            "text-base/6 placeholder:text-zinc-500 sm:text-sm/6 text-white",
            // Border
            "border border-white/10 hover:border-white/20",
            // Background color
            "bg-white/5",
            // Hide default focus styles
            "focus:outline-hidden",
            // Invalid state
            "data-invalid:hover:border-red-500 data-invalid:border-red-500",
            // Disabled state
            "hover:data-disabled:border-white/15 data-disabled:border-white/15 data-disabled:bg-white/2.5",
            // System icons
            "scheme-dark",
          ])}
          data-slot="control"
          placeholder={placeholder}
        />
        <BaseCombobox.Trigger
          aria-label={ariaLabel ?? "Show options"}
          className="group absolute inset-y-0 right-0 flex items-center px-2"
        >
          <ChevronsUpDown
            aria-hidden="true"
            className="size-5 group-data-disabled:text-zinc-600 sm:size-4 text-zinc-400 group-not-data-disabled:group-hover:text-zinc-300 forced-colors:text-[CanvasText]"
          />
        </BaseCombobox.Trigger>
      </span>
      <BaseCombobox.Portal>
        <BaseCombobox.Positioner
          anchor={anchorRef}
          side={anchor}
          align="center"
          sideOffset={8}
          collisionPadding={16}
          className="z-50"
        >
          <BaseCombobox.Popup
            className={cn(
              // Anchor positioning
              "max-h-(--available-height) max-w-(--available-width) data-empty:invisible",
              // Base styles,
              "isolate min-w-[calc(var(--anchor-width)+8px)] select-none scroll-py-1 rounded-xl p-1 empty:invisible",
              // Invisible border that is only visible in `forced-colors` mode for accessibility purposes
              "outline outline-transparent focus:outline-hidden",
              // Handle scrolling when menu won't fit in viewport
              "overflow-y-scroll overscroll-contain",
              // Popover background
              "backdrop-blur-xl bg-zinc-800/75",
              // Shadows
              "shadow-lg ring-1 ring-white/10 ring-inset",
              // Transitions
              "transition-opacity duration-100 ease-in data-ending-style:opacity-0 data-ending-style:pointer-events-none"
            )}
          >
            <BaseCombobox.List>
              {(option: NonNullable<T>) => {
                const child = children(option);
                return cloneElement(child, { key: child.key ?? optionKeys.get(option) });
              }}
            </BaseCombobox.List>
          </BaseCombobox.Popup>
        </BaseCombobox.Positioner>
      </BaseCombobox.Portal>
    </BaseCombobox.Root>
  );
}

export function ComboboxOption<T>({
  children,
  className,
  ...props
}: { className?: string; children?: React.ReactNode } & Omit<
  BaseCombobox.Item.Props & { value: T },
  "as" | "className"
>) {
  const sharedClasses = cn(
    // Base
    "flex min-w-0 items-center",
    // Icons
    "*:data-[slot=icon]:size-5 *:data-[slot=icon]:shrink-0 sm:*:data-[slot=icon]:size-4",
    "group-data-highlighted/option:*:data-[slot=icon]:text-white *:data-[slot=icon]:text-zinc-400",
    "forced-colors:*:data-[slot=icon]:text-[CanvasText] forced-colors:group-data-highlighted/option:*:data-[slot=icon]:text-[Canvas]",
    // Avatars
    "*:data-[slot=avatar]:-mx-0.5 *:data-[slot=avatar]:size-6 sm:*:data-[slot=avatar]:size-5"
  );

  return (
    <BaseCombobox.Item
      {...props}
      className={cn(
        // Basic layout
        "group/option grid w-full cursor-default grid-cols-[1fr_--spacing(5)] items-baseline gap-x-2 rounded-lg py-2.5 pr-2 pl-3.5 sm:grid-cols-[1fr_--spacing(4)] sm:py-1.5 sm:pr-2 sm:pl-3",
        // Typography
        "text-base/6 sm:text-sm/6 text-white forced-colors:text-[CanvasText]",
        // Focus
        "outline-hidden data-highlighted:bg-blue-500 data-highlighted:text-white",
        // Forced colors mode
        "forced-color-adjust-none forced-colors:data-highlighted:bg-[Highlight] forced-colors:data-highlighted:text-[HighlightText]",
        // Disabled
        "data-disabled:opacity-50"
      )}
    >
      <span className={cn(className, sharedClasses)}>{children}</span>
      <Check
        aria-hidden="true"
        className="relative col-start-2 hidden size-5 self-center group-data-selected/option:inline sm:size-4"
      />
    </BaseCombobox.Item>
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
        "flex flex-1 overflow-hidden before:w-2 before:min-w-0 before:shrink group-data-highlighted/option:text-white text-zinc-400"
      )}
    >
      <span className="flex-1 truncate">{children}</span>
    </span>
  );
}

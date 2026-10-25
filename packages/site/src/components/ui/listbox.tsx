"use client";

import { Select } from "@base-ui/react/select";
import { cn } from "cn";
import { Check, ChevronsUpDown } from "lucide-react";
import type React from "react";
import { Children, Fragment, isValidElement } from "react";

type BaseListboxProps<T> = {
  className?: string;
  placeholder?: React.ReactNode;
  autoFocus?: boolean;
  "aria-label"?: string;
  renderValue?: (value: T | T[] | null | undefined) => React.ReactNode;
  children?: React.ReactNode;
} & Omit<Select.Root.Props<T, boolean>, "multiple" | "value" | "onValueChange" | "items">;

type SingleListboxProps<T> = BaseListboxProps<T> & {
  multiple?: false;
  value?: T | null;
  onChange?: (value: T) => void;
};

type MultiListboxProps<T> = BaseListboxProps<T> & {
  multiple: true;
  value?: T[];
  onChange?: (value: T[]) => void;
};

function collectOptions<T>(children: React.ReactNode): { value: T; label: React.ReactNode }[] {
  return Children.toArray(children).flatMap((child) => {
    if (!isValidElement<{ value: T; children?: React.ReactNode; className?: string }>(child))
      return [];
    if (child.type === Fragment) return collectOptions<T>(child.props.children);
    if (child.type !== ListboxOption) return [];
    return [
      {
        value: child.props.value,
        label: (
          <span
            className={cn(
              child.props.className,
              "flex min-w-0 items-center *:data-[slot=icon]:size-5 *:data-[slot=icon]:shrink-0 sm:*:data-[slot=icon]:size-4 *:data-[slot=icon]:text-zinc-400 *:data-[slot=avatar]:-mx-0.5 *:data-[slot=avatar]:size-6 sm:*:data-[slot=avatar]:size-5"
            )}
          >
            {child.props.children}
          </span>
        ),
      },
    ];
  });
}

export function Listbox<T>({
  className,
  placeholder,
  autoFocus,
  "aria-label": ariaLabel,
  multiple = false,
  renderValue,
  value,
  children: options,
  onChange,
  ...props
}: SingleListboxProps<T> | MultiListboxProps<T>) {
  const items = collectOptions<T>(options);
  return (
    <Select.Root<T, boolean>
      {...props}
      items={items}
      value={value}
      multiple={multiple}
      onValueChange={(next) => {
        if (multiple) (onChange as ((value: T[]) => void) | undefined)?.(next as T[]);
        else if (next !== null) (onChange as ((value: T) => void) | undefined)?.(next as T);
      }}
    >
      <Select.Trigger
        data-autofocus={autoFocus || undefined}
        aria-label={ariaLabel}
        autoFocus={autoFocus}
        className={cn([
          className,
          // Basic layout
          "group relative block w-full",
          // Hide default focus styles
          "focus:outline-hidden",
          // Focus ring
          "after:pointer-events-none after:absolute after:inset-0 after:rounded-lg after:ring-transparent after:ring-inset focus-visible:after:ring-2 focus-visible:after:ring-blue-500",
          // Disabled state
          "data-disabled:opacity-50",
        ])}
        data-slot="control"
      >
        <span
          className={cn([
            // Basic layout
            "relative block w-full appearance-none rounded-lg py-[calc(--spacing(2.5)-1px)] sm:py-[calc(--spacing(1.5)-1px)]",
            // Set minimum height for when no value is selected
            "min-h-11 sm:min-h-9",
            // Horizontal padding
            "pr-[calc(--spacing(7)-1px)] pl-[calc(--spacing(3.5)-1px)] sm:pl-[calc(--spacing(3)-1px)]",
            // Typography
            "text-left text-base/6 placeholder:text-zinc-500 sm:text-sm/6 text-white forced-colors:text-[CanvasText]",
            // Border
            "border border-white/10 group-active:border-white/20 group-hover:border-white/20",
            // Background color
            "bg-white/5",
            // Invalid state
            "group-hover:group-data-invalid:border-red-600 group-data-invalid:border-red-600",
            // Disabled state
            "group-data-disabled:opacity-100 group-data-disabled:border-white/15 group-data-disabled:bg-white/2.5 group-data-disabled:group-hover:border-white/15",
          ])}
        >
          <Select.Value
            placeholder={
              placeholder && <span className="block truncate text-zinc-500">{placeholder}</span>
            }
          >
            {renderValue ? (selected) => renderValue(selected) : undefined}
          </Select.Value>
        </span>
        <span className="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-2">
          <ChevronsUpDown
            aria-hidden="true"
            className="size-5 group-data-disabled:text-zinc-600 sm:size-4 text-zinc-400 forced-colors:text-[CanvasText]"
          />
        </span>
      </Select.Trigger>
      <Select.Portal>
        <Select.Positioner align="start" collisionPadding={16} className="z-50">
          <Select.Popup
            className={cn(
              // Anchor positioning
              "max-h-(--available-height) max-w-(--available-width)",
              // Base styles
              "isolate w-max min-w-[calc(var(--anchor-width)+1.75rem)] select-none scroll-py-1 rounded-xl p-1",
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
            {options}
          </Select.Popup>
        </Select.Positioner>
      </Select.Portal>
    </Select.Root>
  );
}

export function ListboxOption<T>({
  children,
  className,
  ...props
}: { className?: string; children?: React.ReactNode } & Omit<
  Select.Item.Props & { value: T },
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
    <Select.Item
      {...props}
      className={cn(
        // Basic layout
        "group/option grid cursor-default grid-cols-[--spacing(5)_1fr] items-baseline gap-x-2 rounded-lg py-2.5 pr-3.5 pl-2 sm:grid-cols-[--spacing(4)_1fr] sm:py-1.5 sm:pr-3 sm:pl-1.5",
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
      <Check
        aria-hidden="true"
        className="relative hidden size-5 self-center group-data-selected/option:inline sm:size-4"
      />
      <Select.ItemText className={cn(className, sharedClasses, "col-start-2")}>
        {children}
      </Select.ItemText>
    </Select.Item>
  );
}

export function ListboxLabel({ className, ...props }: React.ComponentPropsWithoutRef<"span">) {
  return (
    <span
      {...props}
      className={cn(className, "ml-2.5 truncate first:ml-0 sm:ml-2 sm:first:ml-0")}
    />
  );
}

export function ListboxDescription({
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

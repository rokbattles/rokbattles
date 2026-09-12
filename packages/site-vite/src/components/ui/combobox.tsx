import { Combobox as BaseCombobox } from "@base-ui/react/combobox";
import { cn } from "cn";
import { Check, ChevronsUpDown } from "lucide-react";
import type { ComponentProps, ReactNode, Ref } from "react";

type ComboboxProps<Value> = Omit<
  BaseCombobox.Root.Props<Value>,
  "items" | "children" | "multiple" | "itemToStringLabel"
> & {
  options: Value[];
  displayValue: (value: Value | null) => string | undefined;
  children: (value: Value) => ReactNode;
  className?: string;
  placeholder?: string;
  autoFocus?: boolean;
  "aria-label"?: string;
  "aria-labelledby"?: string;
  anchor?: "top" | "bottom";
  ref?: Ref<HTMLInputElement>;
};

export function Combobox<Value>({
  options,
  displayValue,
  filter,
  anchor = "bottom",
  className,
  placeholder,
  autoFocus,
  "aria-label": ariaLabel,
  "aria-labelledby": ariaLabelledby,
  ref,
  children,
  ...props
}: ComboboxProps<Value>) {
  return (
    <BaseCombobox.Root
      {...props}
      items={options}
      itemToStringLabel={(value) => displayValue(value) ?? ""}
      filter={
        filter === undefined
          ? (value, query) =>
              (displayValue(value) ?? "").toLowerCase().includes(query.toLowerCase())
          : filter
      }
    >
      <BaseCombobox.InputGroup
        data-slot="control"
        className={cn("relative block w-full has-data-disabled:opacity-50", className)}
      >
        <BaseCombobox.Input
          ref={ref}
          autoFocus={autoFocus}
          aria-label={ariaLabel}
          aria-labelledby={ariaLabelledby}
          placeholder={placeholder}
          className="relative block w-full appearance-none rounded-lg border border-white/10 bg-white/5 py-[calc(--spacing(2.5)-1px)] pr-[calc(--spacing(10)-1px)] pl-[calc(--spacing(3.5)-1px)] text-base/6 text-white scheme-dark placeholder:text-zinc-500 hover:border-white/20 focus:outline-2 focus:outline-offset-2 focus:outline-blue-500 data-invalid:border-red-600 data-invalid:hover:border-red-600 data-disabled:border-white/15 data-disabled:bg-white/2.5 data-disabled:hover:border-white/15 sm:py-[calc(--spacing(1.5)-1px)] sm:pl-[calc(--spacing(3)-1px)] sm:text-sm/6"
        />
        <BaseCombobox.Trigger className="group absolute inset-y-0 right-0 flex items-center px-2 text-zinc-400 hover:text-zinc-300 data-disabled:text-zinc-600">
          <ChevronsUpDown
            aria-hidden="true"
            className="size-5 sm:size-4 forced-colors:text-[CanvasText]"
          />
        </BaseCombobox.Trigger>
      </BaseCombobox.InputGroup>
      <BaseCombobox.Portal>
        <BaseCombobox.Positioner side={anchor} align="start" sideOffset={8} collisionPadding={16}>
          <BaseCombobox.Popup className="isolate max-h-(--available-height) min-w-(--anchor-width) overflow-y-auto overscroll-contain rounded-xl bg-zinc-800/75 p-1 text-white shadow-lg ring-1 ring-white/10 ring-inset outline outline-transparent backdrop-blur-xl scheme-dark transition-opacity duration-100 focus:outline-hidden data-starting-style:opacity-0 data-ending-style:opacity-0 motion-reduce:transition-none">
            <BaseCombobox.Empty className="px-3 py-2 text-sm text-zinc-400">
              No results found.
            </BaseCombobox.Empty>
            <BaseCombobox.List>{children}</BaseCombobox.List>
          </BaseCombobox.Popup>
        </BaseCombobox.Positioner>
      </BaseCombobox.Portal>
    </BaseCombobox.Root>
  );
}

export function ComboboxOption<Value>({
  children,
  className,
  ...props
}: Omit<ComponentProps<typeof BaseCombobox.Item>, "value" | "className"> & {
  value: Value;
  className?: string;
}) {
  return (
    <BaseCombobox.Item
      {...props}
      className="group/option grid w-full cursor-default grid-cols-[1fr_--spacing(5)] items-baseline gap-x-2 rounded-lg py-2.5 pr-2 pl-3.5 text-base/6 text-white outline-hidden data-highlighted:bg-blue-500 data-highlighted:text-white data-disabled:opacity-50 sm:grid-cols-[1fr_--spacing(4)] sm:py-1.5 sm:pl-3 sm:text-sm/6 forced-color-adjust-none forced-colors:text-[CanvasText] forced-colors:data-highlighted:bg-[Highlight] forced-colors:data-highlighted:text-[HighlightText]"
    >
      <span
        className={cn(
          "flex min-w-0 items-center [&>svg]:size-5 [&>svg]:shrink-0 [&>svg]:text-zinc-400 group-data-highlighted/option:[&>svg]:text-white sm:[&>svg]:size-4 *:data-[slot=avatar]:-mx-0.5 *:data-[slot=avatar]:size-6 sm:*:data-[slot=avatar]:size-5",
          className
        )}
      >
        {children}
      </span>
      <BaseCombobox.ItemIndicator className="col-start-2 self-center">
        <Check aria-hidden="true" className="size-5 sm:size-4" />
      </BaseCombobox.ItemIndicator>
    </BaseCombobox.Item>
  );
}

export {
  OptionDescription as ComboboxDescription,
  OptionLabel as ComboboxLabel,
} from "./option-content";

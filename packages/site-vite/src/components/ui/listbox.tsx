import { Select as BaseSelect } from "@base-ui/react/select";
import clsx from "clsx";
import { Check, ChevronsUpDown } from "lucide-react";
import type { ComponentProps, ReactNode, Ref } from "react";

type ListboxProps<Value> = BaseSelect.Root.Props<Value> & {
  className?: string;
  placeholder?: ReactNode;
  "aria-label"?: string;
  "aria-labelledby"?: string;
  autoFocus?: boolean;
  ref?: Ref<HTMLButtonElement>;
  displayValue?: (value: Value | null) => ReactNode;
};

export function Listbox<Value>({
  className,
  placeholder,
  autoFocus,
  "aria-label": ariaLabel,
  "aria-labelledby": ariaLabelledby,
  ref,
  displayValue,
  children,
  ...props
}: ListboxProps<Value>) {
  return (
    <BaseSelect.Root {...props}>
      <BaseSelect.Trigger
        ref={ref}
        data-slot="control"
        autoFocus={autoFocus}
        aria-label={ariaLabel}
        aria-labelledby={ariaLabelledby}
        className={clsx(
          "group relative block min-h-11 w-full rounded-lg border border-white/10 bg-white/5 py-[calc(--spacing(2.5)-1px)] pr-9 pl-[calc(--spacing(3.5)-1px)] text-left text-base/6 text-white scheme-dark hover:border-white/20 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500 sm:min-h-9 sm:py-[calc(--spacing(1.5)-1px)] sm:pl-[calc(--spacing(3)-1px)] sm:text-sm/6",
          "data-invalid:border-red-600 data-invalid:hover:border-red-600 data-disabled:border-white/15 data-disabled:bg-white/2.5 data-disabled:opacity-50 data-disabled:hover:border-white/15",
          className
        )}
      >
        <BaseSelect.Value
          placeholder={placeholder}
          className="flex min-w-0 items-center truncate data-placeholder:text-zinc-500 [&_svg]:size-5 [&_svg]:shrink-0 [&_svg]:text-zinc-400 sm:[&_svg]:size-4"
        >
          {displayValue}
        </BaseSelect.Value>
        <BaseSelect.Icon className="pointer-events-none absolute inset-y-0 right-2 flex items-center">
          <ChevronsUpDown
            aria-hidden="true"
            className="size-5 text-zinc-400 group-data-disabled:text-zinc-600 sm:size-4 forced-colors:text-[CanvasText]"
          />
        </BaseSelect.Icon>
      </BaseSelect.Trigger>
      <BaseSelect.Portal>
        <BaseSelect.Positioner
          align="start"
          sideOffset={8}
          collisionPadding={16}
          alignItemWithTrigger={false}
        >
          <BaseSelect.Popup className="isolate max-h-(--available-height) min-w-(--anchor-width) overflow-y-auto overscroll-contain rounded-xl bg-zinc-800/75 p-1 text-white shadow-lg ring-1 ring-white/10 ring-inset outline outline-transparent backdrop-blur-xl scheme-dark transition-opacity duration-100 focus:outline-hidden data-starting-style:opacity-0 data-ending-style:opacity-0 motion-reduce:transition-none">
            <BaseSelect.List>{children}</BaseSelect.List>
          </BaseSelect.Popup>
        </BaseSelect.Positioner>
      </BaseSelect.Portal>
    </BaseSelect.Root>
  );
}

export function ListboxOption<Value>({
  children,
  className,
  ...props
}: Omit<ComponentProps<typeof BaseSelect.Item>, "value" | "className"> & {
  value: Value;
  className?: string;
}) {
  return (
    <BaseSelect.Item
      {...props}
      className={clsx(
        "group/option grid cursor-default grid-cols-[--spacing(5)_1fr] items-baseline gap-x-2 rounded-lg py-2.5 pr-3.5 pl-2 text-base/6 text-white outline-hidden sm:grid-cols-[--spacing(4)_1fr] sm:py-1.5 sm:pr-3 sm:pl-1.5 sm:text-sm/6",
        "data-highlighted:bg-blue-500 data-highlighted:text-white data-disabled:opacity-50 forced-color-adjust-none forced-colors:text-[CanvasText] forced-colors:data-highlighted:bg-[Highlight] forced-colors:data-highlighted:text-[HighlightText]"
      )}
    >
      <BaseSelect.ItemIndicator className="col-start-1 row-start-1 self-center">
        <Check aria-hidden="true" className="size-5 sm:size-4" />
      </BaseSelect.ItemIndicator>
      <BaseSelect.ItemText
        className={clsx(
          "col-start-2 row-start-1 flex min-w-0 items-center [&>svg]:size-5 [&>svg]:shrink-0 [&>svg]:text-zinc-400 group-data-highlighted/option:[&>svg]:text-white sm:[&>svg]:size-4 *:data-[slot=avatar]:-mx-0.5 *:data-[slot=avatar]:size-6 sm:*:data-[slot=avatar]:size-5",
          className
        )}
      >
        {children}
      </BaseSelect.ItemText>
    </BaseSelect.Item>
  );
}

export {
  OptionDescription as ListboxDescription,
  OptionLabel as ListboxLabel,
} from "./option-content";

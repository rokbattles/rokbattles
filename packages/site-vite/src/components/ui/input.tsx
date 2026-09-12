import { Input as BaseInput } from "@base-ui/react/input";
import clsx from "clsx";
import type React from "react";

export function InputGroup({ children }: React.ComponentPropsWithoutRef<"span">) {
  return (
    <span
      data-slot="control"
      className={
        "relative isolate block has-[svg:first-child]:[&_input]:pl-10 has-[svg:last-child]:[&_input]:pr-10 sm:has-[svg:first-child]:[&_input]:pl-8 sm:has-[svg:last-child]:[&_input]:pr-8 [&>svg]:pointer-events-none [&>svg]:absolute [&>svg]:top-3 [&>svg]:z-10 [&>svg]:size-5 sm:[&>svg]:top-2.5 sm:[&>svg]:size-4 [&>svg:first-child]:left-3 sm:[&>svg:first-child]:left-2.5 [&>svg:last-child]:right-3 sm:[&>svg:last-child]:right-2.5 [&>svg]:text-zinc-400"
      }
    >
      {children}
    </span>
  );
}

const dateTypes = ["date", "datetime-local", "month", "time", "week"];
type DateType = (typeof dateTypes)[number];

export function Input({
  ref,
  className,
  ...props
}: {
  className?: string;
  type?: "email" | "number" | "password" | "search" | "tel" | "text" | "url" | DateType;
} & Omit<BaseInput.Props, "className"> & { ref?: React.Ref<HTMLInputElement> }) {
  return (
    <span
      data-slot="control"
      className={clsx([
        className,
        "relative block w-full after:pointer-events-none after:absolute after:inset-0 after:rounded-lg after:ring-transparent after:ring-inset sm:focus-within:after:ring-2 sm:focus-within:after:ring-blue-500 has-data-disabled:opacity-50",
      ])}
    >
      <BaseInput
        ref={ref}
        {...props}
        className={clsx([
          props.type &&
            dateTypes.includes(props.type) && [
              "[&::-webkit-datetime-edit-fields-wrapper]:p-0",
              "[&::-webkit-date-and-time-value]:min-h-[1.5em]",
              "[&::-webkit-datetime-edit]:inline-flex",
              "[&::-webkit-datetime-edit]:p-0",
              "[&::-webkit-datetime-edit-year-field]:p-0",
              "[&::-webkit-datetime-edit-month-field]:p-0",
              "[&::-webkit-datetime-edit-day-field]:p-0",
              "[&::-webkit-datetime-edit-hour-field]:p-0",
              "[&::-webkit-datetime-edit-minute-field]:p-0",
              "[&::-webkit-datetime-edit-second-field]:p-0",
              "[&::-webkit-datetime-edit-millisecond-field]:p-0",
              "[&::-webkit-datetime-edit-meridiem-field]:p-0",
            ],
          "relative block w-full appearance-none rounded-lg px-[calc(--spacing(3.5)-1px)] py-[calc(--spacing(2.5)-1px)] sm:px-[calc(--spacing(3)-1px)] sm:py-[calc(--spacing(1.5)-1px)] text-base/6 placeholder:text-zinc-500 sm:text-sm/6 border focus:outline-hidden text-white border-white/10 hover:border-white/20 bg-white/5 data-invalid:border-red-600 data-invalid:hover:border-red-600 data-disabled:border-white/15 data-disabled:bg-white/2.5 hover:data-disabled:border-white/15 scheme-dark",
        ])}
      />
    </span>
  );
}

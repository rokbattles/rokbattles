import clsx from "clsx";
import type { ComponentProps } from "react";

export function OptionLabel({ className, ...props }: ComponentProps<"span">) {
  return (
    <span
      {...props}
      className={clsx("ml-2.5 truncate first:ml-0 sm:ml-2 sm:first:ml-0", className)}
    />
  );
}

export function OptionDescription({ className, children, ...props }: ComponentProps<"span">) {
  return (
    <span
      {...props}
      className={clsx(
        "flex flex-1 overflow-hidden text-zinc-400 group-data-highlighted/option:text-white before:w-2 before:min-w-0 before:shrink",
        className
      )}
    >
      <span className="flex-1 truncate">{children}</span>
    </span>
  );
}

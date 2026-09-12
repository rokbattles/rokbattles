import clsx from "clsx";
import type { ComponentPropsWithoutRef } from "react";

type SubheadingProps = ComponentPropsWithoutRef<"h3"> & { level?: 2 | 3 | 4 | 5 | 6 };

export function Subheading({ level = 3, className, ...props }: SubheadingProps) {
  const Element = `h${level}` as const;
  return (
    <Element
      {...props}
      className={clsx("text-2xl/8 font-medium tracking-tight text-white text-balance", className)}
    />
  );
}

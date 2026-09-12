import clsx from "clsx";
import type { ComponentPropsWithoutRef } from "react";

const sizes = {
  hero: "text-4xl/none tracking-tight sm:text-6xl lg:text-7xl",
  section: "text-3xl/tight tracking-tight sm:text-4xl lg:text-5xl",
  display: "text-4xl/tight tracking-tight sm:text-5xl lg:text-6xl",
};

type HeadingProps = ComponentPropsWithoutRef<"h1"> & {
  level?: 1 | 2 | 3 | 4 | 5 | 6;
  size?: keyof typeof sizes;
};

export function Heading({ level = 2, size = "section", className, ...props }: HeadingProps) {
  const Element = `h${level}` as const;
  return (
    <Element
      {...props}
      className={clsx("text-balance font-medium text-white", sizes[size], className)}
    />
  );
}

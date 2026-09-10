import { Button as BaseButton } from "@base-ui/react/button";
import clsx from "clsx";
import type { ComponentProps, Ref } from "react";
import { Link } from "react-router";

const variants = {
  primary:
    "border-transparent bg-orange-400 text-zinc-950 hover:bg-orange-300 active:bg-orange-500",
  secondary: "border-white/15 bg-transparent text-white hover:bg-white/5 active:bg-white/10",
  plain:
    "border-transparent bg-transparent text-zinc-300 hover:bg-white/5 hover:text-white active:bg-white/10",
};
const sizes = {
  default: "min-h-12 px-5 py-3",
  compact: "min-h-11 px-5 py-2",
  icon: "size-11 p-0",
};

type ButtonProps = {
  variant?: keyof typeof variants;
  size?: keyof typeof sizes;
  className?: string;
  ref?: Ref<HTMLElement>;
} & (
  | ({ href: string } & Omit<ComponentProps<typeof Link>, "to" | "className" | "ref">)
  | ({ href?: never } & Omit<BaseButton.Props, "className" | "ref">)
);

export function Button({
  variant = "secondary",
  size = "default",
  className,
  ref,
  ...props
}: ButtonProps) {
  const classes = clsx(
    "inline-flex shrink-0 touch-manipulation items-center justify-center gap-2 rounded-sm border text-sm/6 font-semibold whitespace-nowrap transition-colors motion-reduce:transition-none focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-orange-400 [&>svg]:shrink-0",
    size === "icon" ? "[&>svg]:size-5" : "[&>svg]:size-4",
    variants[variant],
    sizes[size],
    className
  );

  if (typeof props.href === "string") {
    const { href, ...linkProps } = props;
    return (
      <Link
        reloadDocument={href.startsWith("#")}
        {...linkProps}
        to={href}
        ref={ref as Ref<HTMLAnchorElement>}
        className={classes}
      />
    );
  }

  return (
    <BaseButton
      {...props}
      ref={ref}
      className={clsx(
        classes,
        "cursor-pointer data-disabled:cursor-default data-disabled:opacity-50"
      )}
    />
  );
}

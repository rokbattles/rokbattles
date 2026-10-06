import { cn } from "cn";
import type { ComponentProps, Ref } from "react";
import { SiteLink } from "../site-link";

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

type ButtonElementProps =
  | ({ href: string } & Omit<ComponentProps<typeof SiteLink>, "href" | "className" | "ref">)
  | ({ href?: never } & Omit<ComponentProps<"button">, "className" | "ref">);

type ButtonProps = {
  variant?: keyof typeof variants;
  size?: keyof typeof sizes;
  className?: string;
  ref?: Ref<HTMLElement>;
} & ButtonElementProps;

function isLink(props: ButtonElementProps): props is Extract<ButtonElementProps, { href: string }> {
  return typeof props.href === "string";
}

export function Button({
  variant = "secondary",
  size = "default",
  className,
  ref,
  ...props
}: ButtonProps) {
  const classes = cn(
    "inline-flex shrink-0 touch-manipulation items-center justify-center gap-2 rounded-sm border text-sm/6 font-semibold whitespace-nowrap transition-colors motion-reduce:transition-none focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-orange-400 [&>svg]:shrink-0",
    size === "icon" ? "[&>svg]:size-5" : "[&>svg]:size-4",
    variants[variant],
    sizes[size],
    className
  );

  if (isLink(props)) {
    const { href, ...linkProps } = props;
    return (
      <SiteLink
        {...linkProps}
        href={href}
        ref={ref as Ref<HTMLAnchorElement>}
        className={classes}
      />
    );
  }

  return (
    <button
      type="button"
      {...props}
      ref={ref as Ref<HTMLButtonElement>}
      className={cn(classes, "cursor-pointer disabled:cursor-default disabled:opacity-50")}
    />
  );
}

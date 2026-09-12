import { Button as BaseButton } from "@base-ui/react/button";
import { cn } from "cn";
import type React from "react";
import { Link } from "./link";

const styles = {
  base: "relative isolate inline-flex items-baseline justify-center gap-x-2 rounded-lg border text-base/6 font-semibold px-[calc(--spacing(3.5)-1px)] py-[calc(--spacing(2.5)-1px)] sm:px-[calc(--spacing(3)-1px)] sm:py-[calc(--spacing(1.5)-1px)] sm:text-sm/6 focus:not-focus-visible:outline-hidden focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500 data-disabled:opacity-50 [&>svg]:-mx-0.5 [&>svg]:my-0.5 [&>svg]:size-5 [&>svg]:shrink-0 [&>svg]:self-center [&>svg]:text-(--btn-icon) sm:[&>svg]:my-1 sm:[&>svg]:size-4 forced-colors:[--btn-icon:ButtonText] forced-colors:hover:[--btn-icon:ButtonText]",
  solid:
    "after:absolute after:-z-10 after:shadow-[inset_0_1px_--theme(--color-white/15%)] active:after:bg-(--btn-hover-overlay) hover:after:bg-(--btn-hover-overlay) data-disabled:after:shadow-none bg-(--btn-bg) border-white/5 after:-inset-px after:rounded-lg",
  outline:
    "[--btn-icon:var(--color-zinc-500)] border-white/15 text-white [--btn-bg:transparent] active:bg-white/5 hover:bg-white/5 active:[--btn-icon:var(--color-zinc-400)] hover:[--btn-icon:var(--color-zinc-400)]",
  plain:
    "border-transparent text-white active:bg-white/10 hover:bg-white/10 [--btn-icon:var(--color-zinc-500)] active:[--btn-icon:var(--color-zinc-400)] hover:[--btn-icon:var(--color-zinc-400)]",
  colors: {
    dark: "text-white [--btn-icon:var(--color-zinc-400)] active:[--btn-icon:var(--color-zinc-300)] hover:[--btn-icon:var(--color-zinc-300)] [--btn-hover-overlay:var(--color-white)]/5 [--btn-bg:var(--color-zinc-800)]",
    white:
      "text-zinc-950 [--btn-bg:white] [--btn-icon:var(--color-zinc-400)] active:[--btn-icon:var(--color-zinc-500)] hover:[--btn-icon:var(--color-zinc-500)] [--btn-hover-overlay:var(--color-zinc-950)]/5",
    zinc: "text-white [--btn-bg:var(--color-zinc-600)] [--btn-icon:var(--color-zinc-400)] active:[--btn-icon:var(--color-zinc-300)] hover:[--btn-icon:var(--color-zinc-300)] [--btn-hover-overlay:var(--color-white)]/5",
    indigo:
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-indigo-500)] [--btn-icon:var(--color-indigo-300)] active:[--btn-icon:var(--color-indigo-200)] hover:[--btn-icon:var(--color-indigo-200)]",
    cyan: "text-cyan-950 [--btn-bg:var(--color-cyan-300)] [--btn-hover-overlay:var(--color-white)]/25 [--btn-icon:var(--color-cyan-500)]",
    red: "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-red-600)] [--btn-icon:var(--color-red-300)] active:[--btn-icon:var(--color-red-200)] hover:[--btn-icon:var(--color-red-200)]",
    orange:
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-orange-500)] [--btn-icon:var(--color-orange-300)] active:[--btn-icon:var(--color-orange-200)] hover:[--btn-icon:var(--color-orange-200)]",
    amber:
      "text-amber-950 [--btn-hover-overlay:var(--color-white)]/25 [--btn-bg:var(--color-amber-400)] [--btn-icon:var(--color-amber-600)]",
    yellow:
      "text-yellow-950 [--btn-hover-overlay:var(--color-white)]/25 [--btn-bg:var(--color-yellow-300)] [--btn-icon:var(--color-yellow-600)] active:[--btn-icon:var(--color-yellow-700)] hover:[--btn-icon:var(--color-yellow-700)]",
    lime: "text-lime-950 [--btn-hover-overlay:var(--color-white)]/25 [--btn-bg:var(--color-lime-300)] [--btn-icon:var(--color-lime-600)] active:[--btn-icon:var(--color-lime-700)] hover:[--btn-icon:var(--color-lime-700)]",
    green:
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-green-600)] [--btn-icon:var(--color-white)]/60 active:[--btn-icon:var(--color-white)]/80 hover:[--btn-icon:var(--color-white)]/80",
    emerald:
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-emerald-600)] [--btn-icon:var(--color-white)]/60 active:[--btn-icon:var(--color-white)]/80 hover:[--btn-icon:var(--color-white)]/80",
    teal: "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-teal-600)] [--btn-icon:var(--color-white)]/60 active:[--btn-icon:var(--color-white)]/80 hover:[--btn-icon:var(--color-white)]/80",
    sky: "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-sky-500)] [--btn-icon:var(--color-white)]/60 active:[--btn-icon:var(--color-white)]/80 hover:[--btn-icon:var(--color-white)]/80",
    blue: "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-blue-600)] [--btn-icon:var(--color-blue-400)] active:[--btn-icon:var(--color-blue-300)] hover:[--btn-icon:var(--color-blue-300)]",
    violet:
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-violet-500)] [--btn-icon:var(--color-violet-300)] active:[--btn-icon:var(--color-violet-200)] hover:[--btn-icon:var(--color-violet-200)]",
    purple:
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-purple-500)] [--btn-icon:var(--color-purple-300)] active:[--btn-icon:var(--color-purple-200)] hover:[--btn-icon:var(--color-purple-200)]",
    fuchsia:
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-fuchsia-500)] [--btn-icon:var(--color-fuchsia-300)] active:[--btn-icon:var(--color-fuchsia-200)] hover:[--btn-icon:var(--color-fuchsia-200)]",
    pink: "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-pink-500)] [--btn-icon:var(--color-pink-300)] active:[--btn-icon:var(--color-pink-200)] hover:[--btn-icon:var(--color-pink-200)]",
    rose: "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-rose-500)] [--btn-icon:var(--color-rose-300)] active:[--btn-icon:var(--color-rose-200)] hover:[--btn-icon:var(--color-rose-200)]",
  },
};

type ButtonProps = (
  | { color?: keyof typeof styles.colors; outline?: never; plain?: never }
  | { color?: never; outline: true; plain?: never }
  | { color?: never; outline?: never; plain: true }
) & { className?: string; children?: React.ReactNode } & (
    | ({ href?: never } & Omit<BaseButton.Props, "className">)
    | ({ href: string } & Omit<React.ComponentPropsWithoutRef<typeof Link>, "className">)
  );

export function Button({
  ref,
  color,
  outline,
  plain,
  className,
  children,
  ...props
}: ButtonProps & { ref?: React.Ref<HTMLElement> }) {
  const classes = cn(
    className,
    styles.base,
    outline
      ? styles.outline
      : plain
        ? styles.plain
        : cn(styles.solid, styles.colors[color ?? "zinc"])
  );

  return typeof props.href === "string" ? (
    <Link {...props} className={classes} ref={ref as React.Ref<HTMLAnchorElement>}>
      <TouchTarget>{children}</TouchTarget>
    </Link>
  ) : (
    <BaseButton {...props} className={cn(classes, "cursor-default")} ref={ref}>
      <TouchTarget>{children}</TouchTarget>
    </BaseButton>
  );
}

/**
 * Expand the hit area to at least 44×44px on touch devices
 */
export function TouchTarget({ children }: { children?: React.ReactNode }) {
  return (
    <>
      <span
        className="absolute top-1/2 left-1/2 size-[max(100%,2.75rem)] -translate-x-1/2 -translate-y-1/2 pointer-fine:hidden"
        aria-hidden="true"
      />
      {children}
    </>
  );
}

import {
  Button as HeadlessButton,
  type ButtonProps as HeadlessButtonProps,
} from "@headlessui/react";
import { cn } from "cn";
import type React from "react";
import { forwardRef } from "react";
import { Link } from "./link";

const styles = {
  base: [
    // Base
    "relative isolate inline-flex items-baseline justify-center gap-x-2 rounded-lg border text-base/6 font-semibold",
    // Sizing
    "px-[calc(--spacing(3.5)-1px)] py-[calc(--spacing(2.5)-1px)] sm:px-[calc(--spacing(3)-1px)] sm:py-[calc(--spacing(1.5)-1px)] sm:text-sm/6",
    // Focus
    "focus:not-data-focus:outline-hidden data-focus:outline-2 data-focus:outline-offset-2 data-focus:outline-blue-500",
    // Disabled
    "data-disabled:opacity-50",
    // Icon
    "*:data-[slot=icon]:-mx-0.5 *:data-[slot=icon]:my-0.5 *:data-[slot=icon]:size-5 *:data-[slot=icon]:shrink-0 *:data-[slot=icon]:self-center *:data-[slot=icon]:text-(--btn-icon) sm:*:data-[slot=icon]:my-1 sm:*:data-[slot=icon]:size-4 forced-colors:[--btn-icon:ButtonText] forced-colors:data-hover:[--btn-icon:ButtonText]",
    // Sprites
    "*:data-[slot=sprite]:-mx-0.5 *:data-[slot=sprite]:size-6 *:data-[slot=sprite]:shrink-0 *:data-[slot=sprite]:self-center",
  ],
  solid: [
    // Button background
    "bg-(--btn-bg)",
    // Subtle white outline
    "border-white/5",
    // Shim/overlay, inset to match button foreground and used for hover state + highlight shadow
    "after:absolute after:-z-10",
    // Inner highlight shadow
    "after:shadow-[inset_0_1px_--theme(--color-white/15%)]",
    // White overlay on hover
    "data-active:after:bg-(--btn-hover-overlay) data-hover:after:bg-(--btn-hover-overlay)",
    // Overlay covers the entire button
    "after:-inset-px after:rounded-lg",
    // Disabled
    "data-disabled:after:shadow-none",
  ],
  outline: [
    // Colors
    "border-white/15 text-white [--btn-bg:transparent] data-active:bg-white/5 data-hover:bg-white/5",
    // Icon
    "[--btn-icon:var(--color-zinc-500)] data-active:[--btn-icon:var(--color-zinc-400)] data-hover:[--btn-icon:var(--color-zinc-400)]",
  ],
  plain: [
    // Base
    "border-transparent",
    // Colors
    "text-white data-active:bg-white/10 data-hover:bg-white/10",
    // Icon
    "[--btn-icon:var(--color-zinc-500)] data-active:[--btn-icon:var(--color-zinc-400)] data-hover:[--btn-icon:var(--color-zinc-400)]",
  ],
  colors: {
    "dark/zinc": [
      "text-white [--btn-bg:var(--color-zinc-600)] [--btn-hover-overlay:var(--color-white)]/5",
      "[--btn-icon:var(--color-zinc-400)] data-active:[--btn-icon:var(--color-zinc-300)] data-hover:[--btn-icon:var(--color-zinc-300)]",
    ],
    "dark/white": [
      "text-zinc-950 [--btn-bg:white] [--btn-hover-overlay:var(--color-zinc-950)]/5",
      "[--btn-icon:var(--color-zinc-500)] data-active:[--btn-icon:var(--color-zinc-400)] data-hover:[--btn-icon:var(--color-zinc-400)]",
    ],
    dark: [
      "text-white",
      "[--btn-hover-overlay:var(--color-white)]/5 [--btn-bg:var(--color-zinc-800)]",
      "[--btn-icon:var(--color-zinc-400)] data-active:[--btn-icon:var(--color-zinc-300)] data-hover:[--btn-icon:var(--color-zinc-300)]",
    ],
    white: [
      "text-zinc-950 [--btn-bg:white]",
      "[--btn-hover-overlay:var(--color-zinc-950)]/5",
      "[--btn-icon:var(--color-zinc-400)] data-active:[--btn-icon:var(--color-zinc-500)] data-hover:[--btn-icon:var(--color-zinc-500)]",
    ],
    zinc: [
      "text-white [--btn-bg:var(--color-zinc-600)]",
      "[--btn-hover-overlay:var(--color-white)]/5",
      "[--btn-icon:var(--color-zinc-400)] data-active:[--btn-icon:var(--color-zinc-300)] data-hover:[--btn-icon:var(--color-zinc-300)]",
    ],
    indigo: [
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-indigo-500)]",
      "[--btn-icon:var(--color-indigo-300)] data-active:[--btn-icon:var(--color-indigo-200)] data-hover:[--btn-icon:var(--color-indigo-200)]",
    ],
    cyan: [
      "text-cyan-950 [--btn-bg:var(--color-cyan-300)] [--btn-hover-overlay:var(--color-white)]/25",
      "[--btn-icon:var(--color-cyan-500)]",
    ],
    red: [
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-red-600)]",
      "[--btn-icon:var(--color-red-300)] data-active:[--btn-icon:var(--color-red-200)] data-hover:[--btn-icon:var(--color-red-200)]",
    ],
    orange: [
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-orange-500)]",
      "[--btn-icon:var(--color-orange-300)] data-active:[--btn-icon:var(--color-orange-200)] data-hover:[--btn-icon:var(--color-orange-200)]",
    ],
    amber: [
      "text-amber-950 [--btn-hover-overlay:var(--color-white)]/25 [--btn-bg:var(--color-amber-400)]",
      "[--btn-icon:var(--color-amber-600)]",
    ],
    yellow: [
      "text-yellow-950 [--btn-hover-overlay:var(--color-white)]/25 [--btn-bg:var(--color-yellow-300)]",
      "[--btn-icon:var(--color-yellow-600)] data-active:[--btn-icon:var(--color-yellow-700)] data-hover:[--btn-icon:var(--color-yellow-700)]",
    ],
    lime: [
      "text-lime-950 [--btn-hover-overlay:var(--color-white)]/25 [--btn-bg:var(--color-lime-300)]",
      "[--btn-icon:var(--color-lime-600)] data-active:[--btn-icon:var(--color-lime-700)] data-hover:[--btn-icon:var(--color-lime-700)]",
    ],
    green: [
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-green-600)]",
      "[--btn-icon:var(--color-white)]/60 data-active:[--btn-icon:var(--color-white)]/80 data-hover:[--btn-icon:var(--color-white)]/80",
    ],
    emerald: [
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-emerald-600)]",
      "[--btn-icon:var(--color-white)]/60 data-active:[--btn-icon:var(--color-white)]/80 data-hover:[--btn-icon:var(--color-white)]/80",
    ],
    teal: [
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-teal-600)]",
      "[--btn-icon:var(--color-white)]/60 data-active:[--btn-icon:var(--color-white)]/80 data-hover:[--btn-icon:var(--color-white)]/80",
    ],
    sky: [
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-sky-500)]",
      "[--btn-icon:var(--color-white)]/60 data-active:[--btn-icon:var(--color-white)]/80 data-hover:[--btn-icon:var(--color-white)]/80",
    ],
    blue: [
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-blue-600)]",
      "[--btn-icon:var(--color-blue-400)] data-active:[--btn-icon:var(--color-blue-300)] data-hover:[--btn-icon:var(--color-blue-300)]",
    ],
    violet: [
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-violet-500)]",
      "[--btn-icon:var(--color-violet-300)] data-active:[--btn-icon:var(--color-violet-200)] data-hover:[--btn-icon:var(--color-violet-200)]",
    ],
    purple: [
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-purple-500)]",
      "[--btn-icon:var(--color-purple-300)] data-active:[--btn-icon:var(--color-purple-200)] data-hover:[--btn-icon:var(--color-purple-200)]",
    ],
    fuchsia: [
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-fuchsia-500)]",
      "[--btn-icon:var(--color-fuchsia-300)] data-active:[--btn-icon:var(--color-fuchsia-200)] data-hover:[--btn-icon:var(--color-fuchsia-200)]",
    ],
    pink: [
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-pink-500)]",
      "[--btn-icon:var(--color-pink-300)] data-active:[--btn-icon:var(--color-pink-200)] data-hover:[--btn-icon:var(--color-pink-200)]",
    ],
    rose: [
      "text-white [--btn-hover-overlay:var(--color-white)]/10 [--btn-bg:var(--color-rose-500)]",
      "[--btn-icon:var(--color-rose-300)] data-active:[--btn-icon:var(--color-rose-200)] data-hover:[--btn-icon:var(--color-rose-200)]",
    ],
  },
};

// autoComplete lets callers disable Firefox's restoration of a button's disabled state.
type ButtonProps = (
  | { color?: keyof typeof styles.colors; outline?: never; plain?: never }
  | { color?: never; outline: true; plain?: never }
  | { color?: never; outline?: never; plain: true }
) & { className?: string; children: React.ReactNode } & (
    | (Omit<HeadlessButtonProps, "as" | "className"> & { autoComplete?: "off" })
    | Omit<React.ComponentPropsWithoutRef<typeof Link>, "className">
  );

export const Button = forwardRef(function Button(
  { color, outline, plain, className, children, ...props }: ButtonProps,
  ref: React.ForwardedRef<HTMLElement>
) {
  let variantClasses: string;

  if (outline) {
    variantClasses = cn(styles.outline);
  } else if (plain) {
    variantClasses = cn(styles.plain);
  } else {
    variantClasses = cn(styles.solid, styles.colors[color ?? "dark/zinc"]);
  }

  const classes = cn(className, styles.base, variantClasses);

  return "href" in props ? (
    <Link {...props} className={classes} ref={ref as React.ForwardedRef<HTMLAnchorElement>}>
      <TouchTarget>{children}</TouchTarget>
    </Link>
  ) : (
    <HeadlessButton {...props} className={cn(classes, "cursor-default")} ref={ref}>
      <TouchTarget>{children}</TouchTarget>
    </HeadlessButton>
  );
});

/**
 * Expand the hit area to at least 44×44px on touch devices
 */
export function TouchTarget({ children }: { children: React.ReactNode }) {
  return (
    <>
      <span
        aria-hidden="true"
        className="absolute top-1/2 left-1/2 pointer-fine:hidden size-[max(100%,2.75rem)] -translate-x-1/2 -translate-y-1/2"
      />
      {children}
    </>
  );
}

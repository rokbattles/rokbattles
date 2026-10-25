"use client";

import { Button as BaseButton } from "@base-ui/react/button";
import { cn } from "cn";
import type React from "react";
import { forwardRef } from "react";
import { TouchTarget } from "./button";
import { Link } from "./link";

const colors = {
  red: "bg-red-500/10 text-red-400 group-not-data-disabled:group-hover:bg-red-500/20",
  orange: "bg-orange-500/10 text-orange-400 group-not-data-disabled:group-hover:bg-orange-500/20",
  amber: "bg-amber-400/10 text-amber-400 group-not-data-disabled:group-hover:bg-amber-400/15",
  yellow: "bg-yellow-400/10 text-yellow-300 group-not-data-disabled:group-hover:bg-yellow-400/15",
  lime: "bg-lime-400/10 text-lime-300 group-not-data-disabled:group-hover:bg-lime-400/15",
  green: "bg-green-500/10 text-green-400 group-not-data-disabled:group-hover:bg-green-500/20",
  emerald:
    "bg-emerald-500/10 text-emerald-400 group-not-data-disabled:group-hover:bg-emerald-500/20",
  teal: "bg-teal-500/10 text-teal-300 group-not-data-disabled:group-hover:bg-teal-500/20",
  cyan: "bg-cyan-400/10 text-cyan-300 group-not-data-disabled:group-hover:bg-cyan-400/15",
  sky: "bg-sky-500/10 text-sky-300 group-not-data-disabled:group-hover:bg-sky-500/20",
  blue: "bg-blue-500/15 text-blue-400 group-not-data-disabled:group-hover:bg-blue-500/25",
  indigo: "bg-indigo-500/15 text-indigo-400 group-not-data-disabled:group-hover:bg-indigo-500/20",
  violet: "bg-violet-500/15 text-violet-400 group-not-data-disabled:group-hover:bg-violet-500/20",
  purple: "bg-purple-500/15 text-purple-400 group-not-data-disabled:group-hover:bg-purple-500/20",
  fuchsia:
    "bg-fuchsia-400/10 text-fuchsia-400 group-not-data-disabled:group-hover:bg-fuchsia-400/20",
  pink: "bg-pink-400/10 text-pink-400 group-not-data-disabled:group-hover:bg-pink-400/20",
  rose: "bg-rose-400/10 text-rose-400 group-not-data-disabled:group-hover:bg-rose-400/20",
  zinc: "bg-white/5 text-zinc-400 group-not-data-disabled:group-hover:bg-white/10",
};

interface BadgeProps {
  color?: keyof typeof colors;
}

export function Badge({
  color = "zinc",
  className,
  ...props
}: BadgeProps & React.ComponentPropsWithoutRef<"span">) {
  return (
    <span
      {...props}
      className={cn(
        className,
        "inline-flex items-center gap-x-1.5 rounded-md px-1.5 py-0.5 font-medium text-sm/5 sm:text-xs/5 forced-colors:outline",
        colors[color]
      )}
    />
  );
}

export const BadgeButton = forwardRef(function BadgeButton(
  {
    color = "zinc",
    className,
    children,
    ...props
  }: BadgeProps & { className?: string; children: React.ReactNode } & (
      | Omit<BaseButton.Props, "as" | "className">
      | Omit<React.ComponentPropsWithoutRef<typeof Link>, "className">
    ),
  ref: React.ForwardedRef<HTMLElement>
) {
  const classes = cn(
    className,
    "group relative inline-flex rounded-md focus:not-focus-visible:outline-hidden focus-visible:outline-2 focus-visible:outline-blue-500 focus-visible:outline-offset-2"
  );

  return "href" in props ? (
    <Link {...props} className={classes} ref={ref as React.ForwardedRef<HTMLAnchorElement>}>
      <TouchTarget>
        <Badge color={color}>{children}</Badge>
      </TouchTarget>
    </Link>
  ) : (
    <BaseButton
      data-autofocus={props.autoFocus || undefined}
      {...props}
      className={classes}
      ref={ref}
    >
      <TouchTarget>
        <Badge color={color}>{children}</Badge>
      </TouchTarget>
    </BaseButton>
  );
});

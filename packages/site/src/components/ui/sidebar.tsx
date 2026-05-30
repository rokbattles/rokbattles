"use client";

import {
  Button as HeadlessButton,
  type ButtonProps as HeadlessButtonProps,
  CloseButton as HeadlessCloseButton,
} from "@headlessui/react";
import { LayoutGroup, motion } from "framer-motion";
import type React from "react";
import { forwardRef, useId } from "react";
import { cn } from "@/lib/cn";
import { TouchTarget } from "./button";
import { Link } from "./link";

export function Sidebar({ className, ...props }: React.ComponentPropsWithoutRef<"nav">) {
  return <nav {...props} className={cn(className, "flex h-full min-h-0 flex-col")} />;
}

export function SidebarHeader({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return (
    <div
      {...props}
      className={cn(
        className,
        "flex flex-col border-zinc-950/5 border-b p-4 dark:border-white/5 [&>[data-slot=section]+[data-slot=section]]:mt-2.5"
      )}
    />
  );
}

export function SidebarBody({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return (
    <div
      {...props}
      className={cn(
        className,
        "flex flex-1 flex-col overflow-y-auto p-4 [&>[data-slot=section]+[data-slot=section]]:mt-8"
      )}
    />
  );
}

export function SidebarFooter({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return (
    <div
      {...props}
      className={cn(
        className,
        "flex flex-col border-zinc-950/5 border-t p-4 dark:border-white/5 [&>[data-slot=section]+[data-slot=section]]:mt-2.5"
      )}
    />
  );
}

export function SidebarSection({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  const id = useId();

  return (
    <LayoutGroup id={id}>
      <div {...props} className={cn(className, "flex flex-col gap-0.5")} data-slot="section" />
    </LayoutGroup>
  );
}

export function SidebarDivider({ className, ...props }: React.ComponentPropsWithoutRef<"hr">) {
  return (
    <hr
      {...props}
      className={cn(className, "my-4 border-zinc-950/5 border-t lg:-mx-4 dark:border-white/5")}
    />
  );
}

export function SidebarSpacer({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return <div aria-hidden="true" {...props} className={cn(className, "mt-8 flex-1")} />;
}

export function SidebarHeading({ className, ...props }: React.ComponentPropsWithoutRef<"h3">) {
  return (
    <h3
      {...props}
      className={cn(className, "mb-1 px-2 font-medium text-xs/6 text-zinc-500 dark:text-zinc-400")}
    />
  );
}

export const SidebarItem = forwardRef(function SidebarItem(
  {
    current,
    className,
    children,
    ...props
  }: { current?: boolean; className?: string; children: React.ReactNode } & (
    | Omit<HeadlessButtonProps, "as" | "className">
    | Omit<HeadlessButtonProps<typeof Link>, "as" | "className">
  ),
  ref: React.ForwardedRef<HTMLAnchorElement | HTMLButtonElement>
) {
  const classes = cn(
    // Base
    "flex w-full items-center gap-3 rounded-lg px-2 py-2.5 text-left font-medium text-base/6 text-zinc-950 sm:py-2 sm:text-sm/5",
    // Leading icon/icon-only
    "*:data-[slot=icon]:size-6 *:data-[slot=icon]:shrink-0 *:data-[slot=icon]:fill-zinc-500 sm:*:data-[slot=icon]:size-5",
    // Trailing icon (down chevron or similar)
    "*:last:data-[slot=icon]:ml-auto *:last:data-[slot=icon]:size-5 sm:*:last:data-[slot=icon]:size-4",
    // Avatar
    "*:data-[slot=avatar]:-m-0.5 *:data-[slot=avatar]:size-7 sm:*:data-[slot=avatar]:size-6",
    // Hover
    "data-hover:bg-zinc-950/5 data-hover:*:data-[slot=icon]:fill-zinc-950",
    // Active
    "data-active:bg-zinc-950/5 data-active:*:data-[slot=icon]:fill-zinc-950",
    // Current
    "data-current:*:data-[slot=icon]:fill-zinc-950",
    // Dark mode
    "dark:text-white dark:*:data-[slot=icon]:fill-zinc-400",
    "dark:data-hover:bg-white/5 dark:data-hover:*:data-[slot=icon]:fill-white",
    "dark:data-active:bg-white/5 dark:data-active:*:data-[slot=icon]:fill-white",
    "dark:data-current:*:data-[slot=icon]:fill-white"
  );

  return (
    <span className={cn(className, "relative")}>
      {current && (
        <motion.span
          className="absolute inset-y-2 -left-4 w-0.5 rounded-full bg-zinc-950 dark:bg-white"
          layoutId="current-indicator"
        />
      )}
      {"href" in props ? (
        <HeadlessCloseButton
          as={Link}
          {...props}
          className={classes}
          data-current={current ? "true" : undefined}
          ref={ref}
        >
          <TouchTarget>{children}</TouchTarget>
        </HeadlessCloseButton>
      ) : (
        <HeadlessButton
          {...props}
          className={cn("cursor-default", classes)}
          data-current={current ? "true" : undefined}
          ref={ref}
        >
          <TouchTarget>{children}</TouchTarget>
        </HeadlessButton>
      )}
    </span>
  );
});

export function SidebarLabel({ className, ...props }: React.ComponentPropsWithoutRef<"span">) {
  return <span {...props} className={cn(className, "truncate")} />;
}

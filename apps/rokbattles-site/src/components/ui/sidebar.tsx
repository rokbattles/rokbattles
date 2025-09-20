"use client";

import * as Headless from "@headlessui/react";
import { LayoutGroup, motion } from "motion/react";
import type React from "react";
import { forwardRef, useId } from "react";
import { TouchTarget } from "@/components/ui/button";
import { Link } from "@/components/ui/link";
import { cn } from "@/lib/cn";

export function Sidebar({ className, ...props }: React.ComponentPropsWithoutRef<"nav">) {
  return <nav {...props} className={cn(className, "flex h-full min-h-0 flex-col")} />;
}

export function SidebarHeader({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return (
    <div
      {...props}
      className={cn(
        className,
        "flex flex-col border-b border-zinc-950/5 p-4 dark:border-white/5 [&>[data-slot=section]+[data-slot=section]]:mt-2.5"
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
        "flex flex-col border-t border-zinc-950/5 p-4 dark:border-white/5 [&>[data-slot=section]+[data-slot=section]]:mt-2.5"
      )}
    />
  );
}

export function SidebarSection({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  const id = useId();

  return (
    <LayoutGroup id={id}>
      <div {...props} data-slot="section" className={cn(className, "flex flex-col gap-0.5")} />
    </LayoutGroup>
  );
}

export function SidebarDivider({ className, ...props }: React.ComponentPropsWithoutRef<"hr">) {
  return (
    <hr
      {...props}
      className={cn(className, "my-4 border-t border-zinc-950/5 lg:-mx-4 dark:border-white/5")}
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
      className={cn(className, "mb-1 px-2 text-xs/6 font-medium text-zinc-500 dark:text-zinc-400")}
    />
  );
}

type SidebarItemProps = { current?: boolean; className?: string; children: React.ReactNode };
type SidebarItemLinkProps = Omit<
  React.ComponentProps<typeof Link>,
  "className" | "children" | "href"
> & { href: string };
type SidebarItemButtonProps = Omit<
  Headless.ButtonProps,
  "as" | "className" | "children" | "href"
> & { href?: never };

export const SidebarItem = forwardRef<
  HTMLElement,
  SidebarItemProps & (SidebarItemLinkProps | SidebarItemButtonProps)
>(function SidebarItem(props, ref) {
  const { current, className, children } = props;

  const classes = cn(
    // Base
    "flex w-full items-center gap-3 rounded-lg px-2 py-2.5 text-left text-base/6 font-medium text-zinc-950 sm:py-2 sm:text-sm/5",
    // Leading icon/icon-only
    "*:data-[slot=icon]:size-6 *:data-[slot=icon]:shrink-0 *:data-[slot=icon]:fill-zinc-500 sm:*:data-[slot=icon]:size-5",
    // Trailing icon (down chevron or similar)
    "*:last:data-[slot=icon]:ml-auto *:last:data-[slot=icon]:size-5 sm:*:last:data-[slot=icon]:size-4",
    // Lucide
    "*:data-[slot=lucide]:size-6 *:data-[slot=lucide]:shrink-0 *:data-[slot=lucide]:text-zinc-500 sm:*:data-[slot=lucide]:size-5",
    "*:last:data-[slot=lucide]:ml-auto *:last:data-[slot=lucide]:size-5 sm:*:last:data-[slot=lucide]:size-4",
    // Avatar
    "*:data-[slot=avatar]:-m-0.5 *:data-[slot=avatar]:size-7 sm:*:data-[slot=avatar]:size-6",
    // Hover
    "data-hover:bg-zinc-950/5 data-hover:*:data-[slot=icon]:fill-zinc-950 data-hover:*:data-[slot=lucide]:text-zinc-950",
    // Active
    "data-active:bg-zinc-950/5 data-active:*:data-[slot=icon]:fill-zinc-950 data-active:*:data-[slot=lucide]:text-zinc-950",
    // Current
    "data-current:*:data-[slot=icon]:fill-zinc-950 data-current:*:data-[slot=lucide]:text-zinc-950",
    // Dark mode
    "dark:text-white dark:*:data-[slot=icon]:fill-zinc-400 dark:*:data-[slot=lucide]:text-zinc-400",
    "dark:data-hover:bg-white/5 dark:data-hover:*:data-[slot=icon]:fill-white dark:data-hover:*:data-[slot=lucide]:text-white",
    "dark:data-active:bg-white/5 dark:data-active:*:data-[slot=icon]:fill-white dark:data-active:*:data-[slot=lucide]:text-white",
    "dark:data-current:*:data-[slot=icon]:fill-white dark:data-current:*:data-[slot=lucide]:text-white"
  );

  if ("href" in props && props.href) {
    const rest = props as SidebarItemProps & SidebarItemLinkProps;
    return (
      <span className={cn(className, "relative")}>
        {current && (
          <motion.span
            layoutId="current-indicator"
            className="absolute inset-y-2 -left-4 w-0.5 rounded-full bg-zinc-950 dark:bg-white"
          />
        )}
        <Headless.CloseButton
          // @ts-expect-error overloaded types, runtime works
          as={Link}
          {...rest}
          className={classes}
          data-current={current ? "true" : undefined}
          ref={ref as React.ForwardedRef<HTMLAnchorElement>}
        >
          <TouchTarget>{children}</TouchTarget>
        </Headless.CloseButton>
      </span>
    );
  }

  const rest = props as SidebarItemProps & SidebarItemButtonProps;
  return (
    <span className={cn(className, "relative")}>
      {current && (
        <motion.span
          layoutId="current-indicator"
          className="absolute inset-y-2 -left-4 w-0.5 rounded-full bg-zinc-950 dark:bg-white"
        />
      )}
      <Headless.Button
        {...rest}
        className={cn("cursor-default", classes)}
        data-current={current ? "true" : undefined}
        ref={ref}
      >
        <TouchTarget>{children}</TouchTarget>
      </Headless.Button>
    </span>
  );
});

export function SidebarLabel({ className, ...props }: React.ComponentPropsWithoutRef<"span">) {
  return <span {...props} className={cn(className, "truncate")} />;
}

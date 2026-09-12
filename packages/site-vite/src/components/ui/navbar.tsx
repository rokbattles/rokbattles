import { Button as BaseButton } from "@base-ui/react/button";
import { cn } from "cn";
import * as m from "motion/react-m";
import type React from "react";
import { TouchTarget } from "./button";
import { Link } from "./link";
import { NavigationSection } from "./navigation-section";

export function Navbar({ className, ...props }: React.ComponentPropsWithoutRef<"nav">) {
  return <nav {...props} className={cn(className, "flex flex-1 items-center gap-4 py-2.5")} />;
}

export function NavbarDivider({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return <div aria-hidden="true" {...props} className={cn(className, "h-6 w-px bg-white/10")} />;
}

export function NavbarSection({ className, ...props }: React.ComponentProps<"div">) {
  return <NavigationSection {...props} className={cn(className, "flex items-center gap-3")} />;
}

export function NavbarSpacer({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return <div aria-hidden="true" {...props} className={cn(className, "-ml-4 flex-1")} />;
}

export function NavbarItem({
  ref,
  current,
  className,
  children,
  ...props
}: { current?: boolean; className?: string; children?: React.ReactNode } & (
  | ({ href?: never } & Omit<BaseButton.Props, "className">)
  | ({ href: string } & Omit<React.ComponentPropsWithoutRef<typeof Link>, "className">)
) & { ref?: React.Ref<HTMLAnchorElement | HTMLButtonElement> }) {
  const classes =
    "relative flex min-w-0 items-center gap-3 rounded-lg p-2 text-left text-base/6 font-medium sm:text-sm/5 [&>svg]:size-6 [&>svg]:shrink-0 sm:[&>svg]:size-5 [&>svg:last-child:not(:nth-child(2))]:ml-auto [&>svg:last-child:not(:nth-child(2))]:size-5 sm:[&>svg:last-child:not(:nth-child(2))]:size-4 *:data-[slot=avatar]:-m-0.5 *:data-[slot=avatar]:size-7 *:data-[slot=avatar]:[--avatar-radius:var(--radius-md)] sm:*:data-[slot=avatar]:size-6 text-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500 data-disabled:opacity-50 [&>svg]:text-zinc-400 hover:bg-white/5 hover:[&>svg]:text-white active:bg-white/5 active:[&>svg]:text-white";

  return (
    <span className={cn(className, "relative")}>
      {current && (
        <m.span
          layoutId="current-indicator"
          className="absolute inset-x-2 -bottom-2.5 h-0.5 rounded-full bg-white"
        />
      )}
      {typeof props.href === "string" ? (
        <Link
          {...props}
          className={classes}
          aria-current={current ? "page" : undefined}
          data-current={current ? "true" : undefined}
          ref={ref as React.Ref<HTMLAnchorElement>}
        >
          <TouchTarget>{children}</TouchTarget>
        </Link>
      ) : (
        <BaseButton
          {...props}
          className={cn("cursor-default", classes)}
          aria-current={current ? "page" : undefined}
          data-current={current ? "true" : undefined}
          ref={ref}
        >
          <TouchTarget>{children}</TouchTarget>
        </BaseButton>
      )}
    </span>
  );
}

export function NavbarLabel({ className, ...props }: React.ComponentPropsWithoutRef<"span">) {
  return <span {...props} className={cn(className, "truncate")} />;
}

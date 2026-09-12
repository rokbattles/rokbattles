import { Button as BaseButton } from "@base-ui/react/button";
import clsx from "clsx";
import * as m from "motion/react-m";
import type React from "react";
import { use } from "react";
import { TouchTarget } from "./button";
import { Link } from "./link";
import { NavigationCloseContext } from "./navigation-context";
import { NavigationSection } from "./navigation-section";

export function Sidebar({ className, ...props }: React.ComponentPropsWithoutRef<"nav">) {
  return <nav {...props} className={clsx(className, "flex h-full min-h-0 flex-col")} />;
}

export function SidebarHeader({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return (
    <div
      {...props}
      className={clsx(
        className,
        "flex flex-col border-b p-4 [&>[data-slot=section]+[data-slot=section]]:mt-2.5 border-white/5"
      )}
    />
  );
}

export function SidebarBody({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return (
    <div
      {...props}
      className={clsx(
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
      className={clsx(
        className,
        "flex flex-col border-t p-4 [&>[data-slot=section]+[data-slot=section]]:mt-2.5 border-white/5"
      )}
    />
  );
}

export function SidebarSection({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <NavigationSection
      {...props}
      data-slot="section"
      className={clsx(className, "flex flex-col gap-0.5")}
    />
  );
}

export function SidebarDivider({ className, ...props }: React.ComponentPropsWithoutRef<"hr">) {
  return <hr {...props} className={clsx(className, "my-4 border-t lg:-mx-4 border-white/5")} />;
}

export function SidebarSpacer({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return <div aria-hidden="true" {...props} className={clsx(className, "mt-8 flex-1")} />;
}

export function SidebarHeading({ className, ...props }: React.ComponentPropsWithoutRef<"h3">) {
  return (
    <h3 {...props} className={clsx(className, "mb-1 px-2 text-xs/6 font-medium text-zinc-400")} />
  );
}

export function SidebarItem({
  ref,
  current,
  className,
  children,
  ...props
}: { current?: boolean; className?: string; children?: React.ReactNode } & (
  | ({ href?: never } & Omit<BaseButton.Props, "className">)
  | ({ href: string } & Omit<React.ComponentPropsWithoutRef<typeof Link>, "className">)
) & { ref?: React.Ref<HTMLAnchorElement | HTMLButtonElement> }) {
  const closeNavigation = use(NavigationCloseContext);
  const classes =
    "flex w-full items-center gap-3 rounded-lg px-2 py-2.5 text-left text-base/6 font-medium sm:py-2 sm:text-sm/5 [&>svg]:size-6 [&>svg]:shrink-0 sm:[&>svg]:size-5 [&>svg:last-child]:ml-auto [&>svg:last-child]:size-5 sm:[&>svg:last-child]:size-4 *:data-[slot=avatar]:-m-0.5 *:data-[slot=avatar]:size-7 sm:*:data-[slot=avatar]:size-6 text-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500 data-disabled:opacity-50 [&>svg]:text-zinc-400 hover:bg-white/5 hover:[&>svg]:text-white active:bg-white/5 active:[&>svg]:text-white data-current:[&>svg]:text-white";

  return (
    <span className={clsx(className, "relative")}>
      {current && (
        <m.span
          layoutId="current-indicator"
          className="absolute inset-y-2 -left-4 w-0.5 rounded-full bg-white"
        />
      )}
      {typeof props.href === "string" ? (
        <Link
          {...props}
          onClick={(event) => {
            props.onClick?.(event);
            if (!event.defaultPrevented) closeNavigation?.();
          }}
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
          className={clsx("cursor-default", classes)}
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

export function SidebarLabel({ className, ...props }: React.ComponentPropsWithoutRef<"span">) {
  return <span {...props} className={clsx(className, "truncate")} />;
}

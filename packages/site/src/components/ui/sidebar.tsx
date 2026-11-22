"use client";

import { Button as BaseButton } from "@base-ui/react/button";
import { cn } from "cn";
import { ChevronDown, PanelLeftClose, PanelLeftOpen } from "lucide-react";
import type React from "react";
import { forwardRef, useContext, useId } from "react";
import { TouchTarget } from "./button";
import { Link } from "./link";
import { MobileSidebarCloseContext } from "./mobile-sidebar-context";
import { SidebarContext } from "./sidebar-context";

export function Sidebar({ className, ...props }: React.ComponentPropsWithoutRef<"nav">) {
  return <nav {...props} className={cn(className, "flex h-full min-h-0 flex-col")} />;
}

export function SidebarHeader({
  className,
  children,
  ...props
}: React.ComponentPropsWithoutRef<"div">) {
  const sidebar = useContext(SidebarContext);
  const label = sidebar?.collapsed ? "Expand sidebar" : "Collapse sidebar";

  return (
    <div
      {...props}
      className={cn(
        className,
        "flex items-center gap-2 border-white/5 border-b px-3 py-3 lg:min-h-14 lg:border-white/10 lg:py-1"
      )}
    >
      <div className="min-w-0 flex-1" hidden={sidebar?.collapsed}>
        {children}
      </div>
      {sidebar && (
        <button
          type="button"
          onClick={sidebar.toggle}
          aria-label={label}
          aria-expanded={!sidebar.collapsed}
          title={label}
          className={cn(
            "flex size-8 shrink-0 cursor-pointer items-center justify-center rounded-md text-zinc-400 hover:bg-white/5 hover:text-white focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500",
            sidebar.collapsed && "mx-auto"
          )}
        >
          {sidebar.collapsed ? (
            <PanelLeftOpen className="size-4" aria-hidden="true" />
          ) : (
            <PanelLeftClose className="size-4" aria-hidden="true" />
          )}
        </button>
      )}
    </div>
  );
}

export function SidebarBody({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return (
    <div
      {...props}
      className={cn(className, "flex flex-1 flex-col gap-3 overflow-y-auto px-3 py-2.5")}
    />
  );
}

export function SidebarFooter({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return (
    <div
      {...props}
      className={cn(className, "flex flex-col gap-2.5 border-white/5 border-t px-3 py-2.5")}
    />
  );
}

export function SidebarSection({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return <div {...props} className={cn(className, "flex flex-col gap-0.5")} />;
}

export function SidebarDivider({ className, ...props }: React.ComponentPropsWithoutRef<"hr">) {
  return <hr {...props} className={cn(className, "my-4 border-t lg:-mx-4 border-white/5")} />;
}

export function SidebarSpacer({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return <div aria-hidden="true" {...props} className={cn(className, "flex-1")} />;
}

type SidebarDisclosureProps = {
  label: string;
  open: boolean;
  onToggle: () => void;
};

export function SidebarCategory({
  label,
  open,
  onToggle,
  children,
}: React.PropsWithChildren<SidebarDisclosureProps>) {
  const sidebar = useContext(SidebarContext);
  const id = useId();

  return (
    <SidebarSection>
      {!sidebar?.collapsed && (
        <button
          type="button"
          aria-controls={id}
          aria-expanded={open}
          onClick={onToggle}
          className="group flex min-h-10 cursor-pointer items-center gap-1.5 rounded-md px-2 text-left font-medium text-xs/5 text-zinc-400 hover:bg-white/5 hover:text-white focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-blue-500 lg:min-h-7"
        >
          {label}
          <ChevronDown
            aria-hidden="true"
            className="size-3.5 -rotate-90 transition-transform group-aria-expanded:rotate-0 motion-reduce:transition-none"
          />
        </button>
      )}
      <div id={id} hidden={!open && !sidebar?.collapsed}>
        <SidebarSection>{children}</SidebarSection>
      </div>
    </SidebarSection>
  );
}

export function SidebarSubnav({
  label,
  href,
  icon,
  current,
  open,
  onNavigate,
  onToggle,
  children,
}: React.PropsWithChildren<
  SidebarDisclosureProps & {
    href: string;
    icon: React.ReactNode;
    current: boolean | "location";
    onNavigate: () => void;
  }
>) {
  const sidebar = useContext(SidebarContext);
  const id = useId();
  const showSubnav = open && !sidebar?.collapsed;

  return (
    <SidebarSection>
      <div className="flex items-stretch gap-0.5">
        <SidebarItem
          className="min-w-0 flex-1"
          title={label}
          href={href}
          current={showSubnav ? false : current}
          onNavigate={onNavigate}
        >
          {icon}
          <SidebarLabel>{label}</SidebarLabel>
        </SidebarItem>
        {!sidebar?.collapsed && (
          <button
            type="button"
            aria-label={`${label} subpages`}
            aria-controls={id}
            aria-expanded={open}
            onClick={onToggle}
            className="group flex w-11 shrink-0 cursor-pointer items-center justify-center rounded-md text-zinc-400 hover:bg-white/5 hover:text-white focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-blue-500 lg:w-8"
          >
            <ChevronDown
              aria-hidden="true"
              className="size-3.5 -rotate-90 transition-transform group-aria-expanded:rotate-0 motion-reduce:transition-none"
            />
          </button>
        )}
      </div>
      <div id={id} hidden={!showSubnav} className="ml-4 pl-3">
        <SidebarSection
          className={cn(
            "*:before:pointer-events-none *:before:absolute *:before:top-0 *:before:-left-3 *:before:h-1/2 *:before:w-3 *:before:rounded-bl-lg *:before:border-white/15 *:before:border-b *:before:border-l",
            "*:not-last:after:pointer-events-none *:not-last:after:absolute *:not-last:after:top-[calc(50%-0.5rem)] *:not-last:after:-bottom-0.5 *:not-last:after:-left-3 *:not-last:after:border-white/15 *:not-last:after:border-l"
          )}
        >
          {children}
        </SidebarSection>
      </div>
    </SidebarSection>
  );
}

export const SidebarItem = forwardRef(function SidebarItem(
  {
    current,
    className,
    children,
    ...props
  }: { current?: boolean | "location"; className?: string; children: React.ReactNode } & (
    | Omit<BaseButton.Props, "as" | "className">
    | Omit<React.ComponentPropsWithoutRef<typeof Link>, "as" | "className">
  ),
  ref: React.ForwardedRef<HTMLAnchorElement | HTMLButtonElement>
) {
  const closeSidebar = useContext(MobileSidebarCloseContext);
  const sidebar = useContext(SidebarContext);
  const classes = cn(
    // Base
    "flex min-h-11 w-full items-center gap-2.5 rounded-md px-2 text-left font-medium text-sm/5 lg:min-h-8",
    "focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-blue-500",
    // Leading icon/icon-only
    "*:data-[slot=icon]:size-[18px] *:data-[slot=icon]:shrink-0 lg:*:data-[slot=icon]:size-4",
    // Trailing icon (down chevron or similar)
    "*:last:data-[slot=icon]:ml-auto *:last:data-[slot=icon]:size-4",
    // Avatar
    "*:data-[slot=avatar]:-m-0.5 *:data-[slot=avatar]:size-7 sm:*:data-[slot=avatar]:size-6",
    "text-zinc-300 *:data-[slot=icon]:text-zinc-400",
    "not-data-disabled:hover:bg-white/5 not-data-disabled:hover:*:data-[slot=icon]:text-white",
    "not-data-disabled:active:bg-white/5 not-data-disabled:active:*:data-[slot=icon]:text-white",
    "data-current:bg-white/8 data-current:text-white data-current:ring-1 data-current:ring-white/10 data-current:ring-inset data-current:*:data-[slot=icon]:text-white",
    sidebar?.collapsed && "justify-center *:last:data-[slot=icon]:ml-0"
  );

  return (
    <span className={cn(className, "relative")}>
      {"href" in props ? (
        <Link
          aria-current={current === true ? "page" : current || undefined}
          {...props}
          onClick={(event) => {
            props.onClick?.(event);
            if (!event.defaultPrevented) closeSidebar?.();
          }}
          className={classes}
          data-current={current ? "true" : undefined}
          ref={ref as React.ForwardedRef<HTMLAnchorElement>}
        >
          <TouchTarget>{children}</TouchTarget>
        </Link>
      ) : (
        <BaseButton
          data-autofocus={props.autoFocus || undefined}
          {...props}
          className={cn("cursor-default", classes)}
          data-current={current ? "true" : undefined}
          ref={ref}
        >
          <TouchTarget>{children}</TouchTarget>
        </BaseButton>
      )}
    </span>
  );
});

export function SidebarLabel({ className, ...props }: React.ComponentPropsWithoutRef<"span">) {
  const sidebar = useContext(SidebarContext);
  return <span {...props} className={cn(className, sidebar?.collapsed ? "sr-only" : "truncate")} />;
}

"use client";

import { cn } from "cn";
import { Menu } from "lucide-react";
import type React from "react";
import { useState } from "react";
import { NavbarItem } from "@/components/ui/navbar";
import { MobileSidebar } from "./mobile-sidebar";
import { SidebarContext } from "./sidebar-context";

export function SidebarLayout({
  breadcrumbs,
  headerActions,
  sidebar,
  children,
}: React.PropsWithChildren<{
  breadcrumbs: React.ReactNode;
  headerActions?: React.ReactNode;
  sidebar: React.ReactNode;
}>) {
  const [showSidebar, setShowSidebar] = useState(false);
  const [collapsed, setCollapsed] = useState(false);

  return (
    <div className="relative isolate flex min-h-svh w-full max-lg:flex-col bg-zinc-900 lg:bg-zinc-950">
      {/* Sidebar on desktop */}
      <div className={cn("fixed inset-y-0 left-0 max-lg:hidden", collapsed ? "w-16" : "w-60")}>
        <SidebarContext value={{ collapsed, toggle: () => setCollapsed((value) => !value) }}>
          {sidebar}
        </SidebarContext>
      </div>

      {/* Sidebar on mobile */}
      <MobileSidebar open={showSidebar} close={() => setShowSidebar(false)}>
        {sidebar}
      </MobileSidebar>

      {/* Content */}
      <div
        className={cn(
          "flex min-w-0 flex-1 flex-col [--page-gutter:1rem] lg:py-2 lg:pr-2 lg:[--page-gutter:1.5rem]",
          collapsed ? "lg:pl-16" : "lg:pl-60"
        )}
      >
        <div className="flex min-w-0 grow flex-col lg:rounded-lg lg:bg-zinc-900 lg:shadow-xs lg:ring-1 lg:ring-white/10">
          <header className="flex min-h-14 items-center gap-3 border-white/10 border-b bg-zinc-950/40 px-(--page-gutter) py-2 lg:rounded-t-lg lg:gap-4">
            <NavbarItem
              className="shrink-0 lg:hidden"
              onClick={() => setShowSidebar(true)}
              aria-label="Open navigation"
            >
              <Menu data-slot="icon" aria-hidden="true" />
            </NavbarItem>
            <div className="min-w-0 flex-1">{breadcrumbs}</div>
            {headerActions && (
              <div className="flex shrink-0 items-center gap-2">{headerActions}</div>
            )}
          </header>
          <main className="min-w-0 grow p-(--page-gutter)">{children}</main>
        </div>
      </div>
    </div>
  );
}

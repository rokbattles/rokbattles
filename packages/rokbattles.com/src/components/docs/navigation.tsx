"use client";

import { cn } from "cn";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { installationDocs } from "@/content/metadata";

function NavLink({ href, children }: { href: string; children: string }) {
  const pathname = usePathname().replace(/\/$/, "");
  const active = pathname === href;

  return (
    <Link
      href={href}
      aria-current={active ? "page" : undefined}
      className={cn(
        "flex min-h-11 items-center rounded-lg px-3 text-sm focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-orange-400",
        active ? "bg-white/5 text-orange-400" : "text-zinc-400 hover:bg-white/5 hover:text-white"
      )}
    >
      {children}
    </Link>
  );
}

export function DocsNavigation() {
  return (
    <nav aria-label="Documentation" className="md:sticky md:top-8">
      <NavLink href="/docs">Overview</NavLink>
      <p className="mt-6 mb-2 px-3 text-sm font-medium text-white">Installation</p>
      <div className="grid grid-cols-2 gap-1 md:grid-cols-1">
        {installationDocs.map(({ slug, title }) => (
          <NavLink key={slug} href={`/docs/installation/${slug}`}>
            {title}
          </NavLink>
        ))}
      </div>
    </nav>
  );
}

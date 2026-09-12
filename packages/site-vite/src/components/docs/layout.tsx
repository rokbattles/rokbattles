import { cn } from "cn";
import { NavLink, Outlet } from "react-router";
import { installationDocs } from "../../lib/docs";
import { gutter } from "../ui/marketing/layout";

const navigationClass = ({ isActive }: { isActive: boolean }) =>
  cn(
    "flex min-h-11 items-center rounded-lg px-3 text-sm focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-orange-400",
    isActive ? "bg-white/5 text-orange-400" : "text-zinc-400 hover:bg-white/5 hover:text-white"
  );

export function DocsLayout() {
  return (
    <div className="grid flex-1 content-start border-b border-white/10 md:grid-cols-[15rem_minmax(0,1fr)] md:content-stretch">
      <aside
        className={cn(gutter, "border-b border-white/10 py-6 md:border-r md:border-b-0 md:py-12")}
      >
        <nav aria-label="Documentation" className="md:sticky md:top-8">
          <NavLink to="/docs" end className={navigationClass}>
            Overview
          </NavLink>
          <p className="mt-6 mb-2 px-3 text-sm font-medium text-white">Installation</p>
          <div className="grid grid-cols-2 gap-1 md:grid-cols-1">
            {installationDocs.map(({ slug, title }) => (
              <NavLink key={slug} to={`/docs/installation/${slug}`} className={navigationClass}>
                {title}
              </NavLink>
            ))}
          </div>
        </nav>
      </aside>
      <div className={cn(gutter, "min-w-0 py-10 sm:py-12 lg:px-12")}>
        <div className="mx-auto max-w-3xl">
          <Outlet />
        </div>
      </div>
    </div>
  );
}

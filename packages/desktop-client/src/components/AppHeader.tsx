import type { ReactNode } from "react";
import { NavLink } from "react-router";

type AppHeaderProps = {
  version: string | null;
};

const navLinkClasses =
  "relative rounded-lg px-2 py-1.5 text-sm/5 font-medium text-zinc-300 hover:bg-white/5 hover:text-white";

export function AppHeader({ version }: AppHeaderProps): ReactNode {
  return (
    <header className="flex flex-wrap items-center justify-between gap-4 border-b border-white/10 py-4">
      <div className="flex min-w-0 items-center gap-3">
        <h1 className="truncate text-lg/7 font-semibold text-white">ROK Battles</h1>
        {version ? (
          <span className="inline-flex items-center rounded-md bg-white/5 px-1.5 py-0.5 text-xs/5 font-medium text-zinc-400">
            {version}
          </span>
        ) : null}
      </div>
      <nav className="flex items-center gap-3">
        <NavLink to="/" className={navLinkClasses}>
          {({ isActive }) => (
            <>
              Home
              {isActive ? (
                <span className="absolute inset-x-2 -bottom-[17px] h-0.5 rounded-full bg-white" />
              ) : null}
            </>
          )}
        </NavLink>
        <NavLink to="/settings" className={navLinkClasses}>
          {({ isActive }) => (
            <>
              Settings
              {isActive ? (
                <span className="absolute inset-x-2 -bottom-[17px] h-0.5 rounded-full bg-white" />
              ) : null}
            </>
          )}
        </NavLink>
      </nav>
    </header>
  );
}

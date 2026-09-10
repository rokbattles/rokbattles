import { Outlet } from "react-router";
import { MarketingFooter } from "./marketing/footer";
import { MarketingHeader } from "./marketing/header";
import { frame } from "./ui/marketing/layout";

export function MarketingLayout() {
  return (
    <div className="min-h-svh bg-zinc-950 text-white selection:bg-orange-400 selection:text-zinc-950">
      <a
        href="#main"
        className="sr-only focus:fixed focus:top-4 focus:left-4 focus:z-50 focus:m-0 focus:h-auto focus:w-auto focus:overflow-visible focus:rounded-sm focus:bg-zinc-900 focus:px-4 focus:py-3 focus:[clip:auto] focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-orange-400"
      >
        Skip to content
      </a>
      <MarketingHeader />
      <div className={frame}>
        <main id="main">
          <Outlet />
        </main>
        <MarketingFooter />
      </div>
    </div>
  );
}

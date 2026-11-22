import { ChevronRight } from "lucide-react";
import { useSearchParams } from "next/navigation";
import { Suspense } from "react";
import { Link } from "@/components/ui/link";
import { accountReportLinkFrom, accountReportLinks } from "@/lib/account-reports-source";

type BreadcrumbRoute = {
  label: string;
  href?: string | null;
};

type Breadcrumb = Omit<BreadcrumbRoute, "href"> & { path: string; href: string | null };

const routes: Record<string, BreadcrumbRoute | undefined> = {
  "/report": { label: "Battle Reports", href: "/" },
  "/report/*": { label: "Battle Report" },
  "/olympian-arena": { label: "Olympian Arena" },
  "/olympian-arena/*": { label: "Battle Report" },
  "/combat-lab": { label: "Combat Lab" },
  "/combat-lab/rankings": { label: "Rankings" },
  "/combat-lab/presoc": { label: "Pre-SoC" },
  "/combat-lab/presoc/rankings": { label: "Rankings" },
  "/territory-lab": { label: "Territory Lab" },
  "/loot-explorer": { label: "Loot Explorer", href: null },
  "/loot-explorer/barbarians": { label: "Barbarians" },
  "/loot-explorer/barbarian-forts": { label: "Barbarian Forts" },
  "/loot-explorer/baulurs": { label: "Baulurs" },
  "/loot-explorer/kahars-treasure": { label: "Kahar's Treasure" },
  "/loot-explorer/karuak-ceremony": { label: "Karuak Ceremony" },
  "/loot-explorer/lohars-trial": { label: "Lohar's Trial" },
  "/loot-explorer/strange-incidents": { label: "Strange Incidents" },
  "/account": { label: "Account", href: null },
  ...Object.fromEntries(accountReportLinks.map(({ href, label }) => [href, { label }])),
  "/account/pairings": { label: "Pairings" },
  "/account/pairings/*": { label: "Pairing Details" },
  "/account/loot": { label: "Loot", href: null },
  "/account/loot/barbarians": { label: "Barbarians" },
  "/account/loot/barbarian-forts": { label: "Barbarian Forts" },
  "/account/loot/baulurs": { label: "Baulurs" },
  "/account/loot/kahars-treasure": { label: "Kahar's Treasure" },
  "/account/loot/karuak-ceremony": { label: "Karuak Ceremony" },
  "/account/loot/lohars-trial": { label: "Lohar's Trial" },
  "/account/resources": { label: "Gathering" },
  "/account/trading-post": { label: "Trading Post" },
  "/account/ark": { label: "Ark Recap" },
  "/account/ark/*": { label: "Ark Match" },
  "/account/settings": { label: "Settings" },
  "/account/settings/apps": { label: "Connected Apps" },
  "/account/settings/devices": { label: "Devices" },
  "/account/settings/game": { label: "Game" },
  "/account/settings/game/governors": { label: "Governor Binds" },
  "/creators": { label: "Content Creators", href: null },
  "/creators/overlay": { label: "Stream Overlay" },
  "/developer": { label: "Developer Program", href: null },
  "/developer/apps": { label: "Apps" },
  "/developer/apps/*": { label: "App Settings" },
  "/developer/apps/*/scopes": { label: "Scopes" },
  "/developer/apps/*/authorization": { label: "Authorization" },
  "/admin": { label: "Admin", href: null },
  "/admin/program-applications": { label: "Program Applications" },
};

function getBreadcrumbs(pathname: string): Breadcrumb[] {
  if (pathname === "/") {
    return [{ path: "/", label: "Battle Reports", href: "/" }];
  }

  const breadcrumbs: Breadcrumb[] = [];
  let path = "";
  let pattern = "";

  for (const segment of pathname.slice(1).split("/")) {
    path += `/${segment}`;
    const exact = `${pattern}/${segment}`;
    pattern = routes[exact] ? exact : `${pattern}/*`;
    const route = routes[pattern];
    if (!route) return [];
    breadcrumbs.push({ ...route, path, href: route.href === undefined ? path : route.href });
  }

  return breadcrumbs;
}

export function PlatformBreadcrumbs({ pathname }: { pathname: string }) {
  const breadcrumbs = getBreadcrumbs(pathname);
  if (breadcrumbs.length === 0) return null;

  if (pathname.startsWith("/report/") || pathname.startsWith("/account/pairings/")) {
    return (
      <Suspense fallback={<BreadcrumbTrail breadcrumbs={breadcrumbs} />}>
        <DetailBreadcrumbs breadcrumbs={breadcrumbs} />
      </Suspense>
    );
  }

  return <BreadcrumbTrail breadcrumbs={breadcrumbs} />;
}

function DetailBreadcrumbs({ breadcrumbs }: { breadcrumbs: Breadcrumb[] }) {
  const searchParams = useSearchParams();
  const current = breadcrumbs[breadcrumbs.length - 1];
  let parents = breadcrumbs.slice(0, -1);
  let query = new URLSearchParams(searchParams);

  if (current.path.startsWith("/report/")) {
    const source = query.get("from");
    const matchId = source === "ark" ? query.get("matchId") : null;

    if (matchId) {
      parents = getBreadcrumbs(`/account/ark/${encodeURIComponent(matchId)}`);
      query = new URLSearchParams({ tab: "reports" });
    } else {
      const reportList = accountReportLinkFrom(source ?? undefined);
      parents = getBreadcrumbs(reportList?.href ?? "/");
      query.delete("from");
    }
  } else {
    query.delete("tab");
    query.delete("loadout");
  }

  const parent = parents[parents.length - 1];
  if (query.size && parent.href) {
    parents[parents.length - 1] = { ...parent, href: `${parent.href}?${query}` };
  }

  return <BreadcrumbTrail breadcrumbs={[...parents, current]} />;
}

function BreadcrumbTrail({ breadcrumbs }: { breadcrumbs: Breadcrumb[] }) {
  return (
    <nav aria-label="Breadcrumb">
      <ol className="flex flex-wrap items-center gap-x-2.5 gap-y-1 text-sm/5 text-zinc-400">
        {breadcrumbs.map(({ path, label, href }, index) => {
          const current = index === breadcrumbs.length - 1;

          return (
            <li
              key={path}
              aria-current={current ? "page" : undefined}
              className="flex items-center gap-2.5 aria-[current=page]:font-medium aria-[current=page]:text-zinc-100"
            >
              {index > 0 && <ChevronRight className="size-3 shrink-0 text-zinc-500" aria-hidden />}
              {href && !current ? (
                <Link
                  href={href}
                  className="rounded-sm hover:text-white hover:underline hover:underline-offset-4 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-400"
                >
                  {label}
                </Link>
              ) : (
                <span>{label}</span>
              )}
            </li>
          );
        })}
      </ol>
    </nav>
  );
}

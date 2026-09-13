import { PALETTE, type Plan, type Route } from "./types";

/** The selected starting site is always the first displayed and calculated stop. */
export function orderedRouteSites(route: Route): string[] {
  return route.startSiteId && route.siteIds.includes(route.startSiteId)
    ? [route.startSiteId, ...route.siteIds.filter((id) => id !== route.startSiteId)]
    : route.siteIds;
}

export function withRouteSites(route: Route, siteIds: string[]): Route {
  return { ...route, siteIds, startSiteId: siteIds[0] };
}

/** Clicking a site in another route transfers it, keeping share references unique. */
export function toggleRouteSite(routes: Route[], routeId: string, siteId: string): Route[] {
  return routes.map((route) => {
    const ids = orderedRouteSites(route);
    return withRouteSites(
      route,
      route.id === routeId && !ids.includes(siteId)
        ? [...ids, siteId]
        : ids.filter((id) => id !== siteId)
    );
  });
}

export function toggleRoutePass(
  routes: Route[],
  routeId: string,
  siteId: string,
  allianceId: string
): Route[] {
  return routes.map((route) => ({
    ...route,
    passes:
      route.id === routeId && !route.passes.some((pass) => pass.siteId === siteId)
        ? [...route.passes, { siteId, allianceId }]
        : route.passes.filter((pass) => pass.siteId !== siteId),
  }));
}

/** Keep existing route identities when opening a share, then fill its unused slots. */
export function preparePlan(plan: Plan): Plan {
  if (plan.mode !== "baulur") return plan;
  const colors = PALETTE.filter((color) => !plan.routes.some((route) => route.color === color));
  for (let index = colors.length - 1; index > 0; index--) {
    const other = Math.floor(Math.random() * (index + 1));
    [colors[index], colors[other]] = [colors[other], colors[index]];
  }
  const routes = plan.routes.map((route, index) => ({ ...route, name: `Route ${index + 1}` }));
  while (routes.length < 5) {
    routes.push({
      id: crypto.randomUUID(),
      name: `Route ${routes.length + 1}`,
      color: colors[routes.length - plan.routes.length],
      commander: "",
      siteIds: [],
      passes: [],
    });
  }
  return { ...plan, annotations: [], routes };
}

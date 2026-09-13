import { Button } from "@/components/ui/button";
import { Field, Label } from "@/components/ui/fieldset";
import { Subheading } from "@/components/ui/heading";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/listbox";
import { Text } from "@/components/ui/text";
import { orderedRouteSites, withRouteSites } from "@/lib/territory-lab/routes";
import type { MapData, Plan, Route, Site } from "@/lib/territory-lab/types";

function siteName(site: Site | undefined) {
  const name = site?.label ?? (site?.kind === "barbarian-keep" ? "Keep" : "Camp");
  return site
    ? `${name} · ${Math.round(site.x / 6)}, ${Math.round(site.y / 6)}`
    : "Site unavailable";
}

export function BaulurDetails({
  plan,
  data,
  onUpdate,
}: {
  plan: Plan;
  data: MapData;
  onUpdate: (id: string, update: (route: Route) => Route) => void;
}) {
  const sites = new Map(data.sites.map((site) => [site.id, site]));
  return (
    <section aria-label="Routes and assigned passes" className="space-y-6">
      <Subheading>Routes</Subheading>
      {plan.routes.map((route) => (
        <section
          key={route.id}
          aria-label={route.name}
          className="border-t border-zinc-950/10 pt-5 dark:border-white/10"
        >
          <div className="flex flex-wrap items-center justify-between gap-3">
            <div>
              <Subheading className="flex items-center gap-2">
                <span
                  aria-hidden="true"
                  className="size-3 shrink-0 rounded-full"
                  style={{ backgroundColor: route.color }}
                />
                {route.name}
              </Subheading>
              <Text>{route.commander || "No commander selected."}</Text>
            </div>
          </div>
          <div className="mt-4 grid gap-6 xl:grid-cols-2">
            <div className="space-y-3">
              <Subheading>Stops · {route.siteIds.length}</Subheading>
              {route.siteIds.length > 0 ? (
                <>
                  <Field className="max-w-sm">
                    <Label>Starting point for {route.name}</Label>
                    <Listbox<string>
                      value={route.startSiteId ?? route.siteIds[0]}
                      onChange={(selectedId) =>
                        onUpdate(route.id, (current) =>
                          withRouteSites(current, [
                            selectedId,
                            ...current.siteIds.filter((id) => id !== selectedId),
                          ])
                        )
                      }
                    >
                      {route.siteIds.map((id) => (
                        <ListboxOption key={id} value={id}>
                          <ListboxLabel>{siteName(sites.get(id))}</ListboxLabel>
                        </ListboxOption>
                      ))}
                    </Listbox>
                  </Field>
                  <ol className="space-y-2 text-sm text-zinc-700 dark:text-zinc-300">
                    {orderedRouteSites(route).map((id, index) => (
                      <li key={id} className="flex flex-wrap items-center gap-2">
                        <span className="min-w-0 flex-1">
                          {index + 1}. {siteName(sites.get(id))}
                        </span>
                        <Button
                          outline
                          aria-label={`Move stop ${index + 1} earlier in ${route.name}`}
                          disabled={index === 0}
                          onClick={() =>
                            onUpdate(route.id, (current) => {
                              const ids = [...orderedRouteSites(current)];
                              [ids[index - 1], ids[index]] = [ids[index], ids[index - 1]];
                              return withRouteSites(current, ids);
                            })
                          }
                        >
                          ↑
                        </Button>
                        <Button
                          outline
                          aria-label={`Remove stop ${index + 1} from ${route.name}`}
                          onClick={() =>
                            onUpdate(route.id, (current) =>
                              withRouteSites(
                                current,
                                orderedRouteSites(current).filter((siteId) => siteId !== id)
                              )
                            )
                          }
                        >
                          ×
                        </Button>
                      </li>
                    ))}
                  </ol>
                </>
              ) : (
                <Text>Click a camp or keep to add the first stop.</Text>
              )}
            </div>
            <div className="space-y-3">
              <Subheading>Assigned passes · {route.passes.length}</Subheading>
              {!route.passes.length && (
                <Text>Click a pass to assign it to this route and the active alliance.</Text>
              )}
              {route.passes.map((pass) => (
                <div key={pass.siteId} className="flex flex-wrap items-end gap-3">
                  <Field className="min-w-48 flex-1">
                    <Label>{siteName(sites.get(pass.siteId))}</Label>
                    <Listbox<string>
                      aria-label={`Alliance for ${siteName(sites.get(pass.siteId))} in ${route.name}`}
                      value={pass.allianceId}
                      onChange={(selectedId) =>
                        onUpdate(route.id, (current) => ({
                          ...current,
                          passes: current.passes.map((value) =>
                            value.siteId === pass.siteId
                              ? { ...value, allianceId: selectedId }
                              : value
                          ),
                        }))
                      }
                    >
                      {plan.alliances.map((alliance) => (
                        <ListboxOption key={alliance.id} value={alliance.id}>
                          <ListboxLabel>{alliance.name}</ListboxLabel>
                        </ListboxOption>
                      ))}
                    </Listbox>
                  </Field>
                  <Button
                    outline
                    aria-label={`Remove ${siteName(sites.get(pass.siteId))} from ${route.name}`}
                    onClick={() =>
                      onUpdate(route.id, (current) => ({
                        ...current,
                        passes: current.passes.filter((value) => value.siteId !== pass.siteId),
                      }))
                    }
                  >
                    Remove pass
                  </Button>
                </div>
              ))}
            </div>
          </div>
        </section>
      ))}
    </section>
  );
}

"use client";

import dynamic from "next/dynamic";
import { useEffect, useState } from "react";
import { preparePlan } from "@/lib/territory-lab/routes";
import {
  API,
  type Catalog,
  type MapSummary,
  type Mode,
  newPlan,
  type Plan,
} from "@/lib/territory-lab/types";
import { MapPicker } from "./map-picker";

const Editor = dynamic(() => import("./editor"), {
  loading: () => <p className="py-12 text-zinc-500">Preparing planner...</p>,
});

type Session = { catalog: Catalog; plan: Plan };

async function loadMaps(signal?: AbortSignal): Promise<MapSummary[]> {
  const response = await fetch(`${API}/list`, { signal, cache: "no-cache" });
  if (!response.ok) throw new Error("Could not load the map catalog.");
  const list = await response.json();
  return list.maps;
}

export function TerritoryLab() {
  const [maps, setMaps] = useState<MapSummary[]>([]);
  const [session, setSession] = useState<Session | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  useEffect(() => {
    const abort = new AbortController();
    (async () => {
      const maps = await loadMaps(abort.signal);
      setMaps(maps);
      const query = new URLSearchParams(location.search);
      const sharedId = query.get("share");
      if (sharedId) {
        const shared = await fetch(`${API}/share/${encodeURIComponent(sharedId)}`, {
          signal: abort.signal,
        });
        if (!shared.ok) throw new Error("This shared plan could not be found.");
        const { plan } = await shared.json();
        const config = await fetch(`${API}/map/${encodeURIComponent(plan.map)}`, {
          signal: abort.signal,
          cache: "no-cache",
        });
        if (!config.ok) throw new Error("The map for this plan is unavailable.");
        setSession({ catalog: await config.json(), plan: preparePlan(plan) });
      } else if (query.get("map")) {
        const map = maps.find((m) => m.map === query.get("map"));
        const mode = query.get("mode") === "baulur" ? "baulur" : "territory";
        if (map && (mode === "territory" || map.season === "preparation")) {
          const config = await fetch(`${API}/map/${encodeURIComponent(map.map)}`, {
            signal: abort.signal,
            cache: "no-cache",
          });
          if (!config.ok) throw new Error("This map could not be loaded.");
          setSession({ catalog: await config.json(), plan: preparePlan(newPlan(map.map, mode)) });
        }
      }
    })()
      .catch((e) => {
        if (!abort.signal.aborted) setError(String(e));
      })
      .finally(() => {
        if (!abort.signal.aborted) setLoading(false);
      });
    return () => abort.abort();
  }, []);

  async function showMaps() {
    setLoading(true);
    setError("");
    try {
      // The editor can stay open while map names or availability change.
      setMaps(await loadMaps());
      setSession(null);
      history.replaceState(null, "", "/territory-lab");
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function open(map: string, mode: Mode) {
    setLoading(true);
    setError("");
    try {
      const response = await fetch(`${API}/map/${encodeURIComponent(map)}`, { cache: "no-cache" });
      if (!response.ok) throw new Error("This map could not be loaded.");
      const catalog: Catalog = await response.json();
      if (mode === "baulur" && catalog.season !== "preparation")
        throw new Error("Baulur routes are available only on home kingdom maps.");
      setSession({ catalog, plan: preparePlan(newPlan(map, mode)) });
      const url = new URL(location.href);
      url.searchParams.delete("share");
      url.searchParams.set("map", map);
      url.searchParams.set("mode", mode);
      history.replaceState(null, "", url);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }
  if (loading)
    return (
      <p className="py-12 text-zinc-500" role="status">
        Loading Territory Lab...
      </p>
    );
  return (
    <>
      {error && (
        <p className="border-l-3 border-red-500 px-4 py-[0.6rem]" role="alert">
          {error}
        </p>
      )}
      {session ? (
        <Editor
          key={`${session.catalog.map}/${session.plan.mode}`}
          catalog={session.catalog}
          initialPlan={session.plan}
          back={showMaps}
        />
      ) : (
        <MapPicker maps={maps} open={open} />
      )}
    </>
  );
}

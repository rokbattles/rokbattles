"use client";

import { ChevronLeftIcon } from "@heroicons/react/20/solid";
import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { Checkbox, CheckboxField, CheckboxGroup } from "@/components/ui/checkbox";
import { Combobox, ComboboxLabel, ComboboxOption } from "@/components/ui/combobox";
import { Field, Label } from "@/components/ui/fieldset";
import { Heading, Subheading } from "@/components/ui/heading";
import { Input } from "@/components/ui/input";
import { Link } from "@/components/ui/link";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/listbox";
import { Text } from "@/components/ui/text";
import { useCommanderOptions } from "@/hooks/use-commander-name";
import { updatePlannerSelection } from "@/lib/territory/selection";
import { loadMapData } from "@/lib/territory-lab/data";
import { toggleRoutePass, toggleRouteSite } from "@/lib/territory-lab/routes";
import { placementError, territorySummary } from "@/lib/territory-lab/territory";
import {
  type Annotation,
  API,
  type BuildingKind,
  type Catalog,
  type MapData,
  PALETTE,
  type Plan,
  type Point,
  type Route,
  type RouteLeg,
  type Site,
  type Tool,
} from "@/lib/territory-lab/types";
import { BaulurDetails } from "./baulur-details";
import { type CameraStatus, LabController, type MapVisibility } from "./controller";
import { TerritoryDetails } from "./territory-details";

const TOOL_LABELS: [Tool, string][] = [
  ["select", "Select"],
  ["line", "Line"],
  ["arrow", "Arrow"],
  ["circle", "Circle"],
  ["marker", "Marker"],
  ["text", "Text"],
];
const BUILDING_LABELS: [BuildingKind, string][] = [
  ["center-fortress", "Center fortress"],
  ["fortress", "Alliance fortress"],
  ["horse", "Alliance horse"],
  ["flag", "Flag"],
];
const COLOR_NAMES = ["Red", "Orange", "Yellow", "Green", "Cyan", "Blue", "Purple", "Pink"];

function Palette({ color, change }: { color: string; change: (color: string) => void }) {
  return (
    <div className="my-[0.65rem] flex flex-wrap gap-[0.6rem]">
      {PALETTE.map((value, i) => (
        <Button
          plain
          key={value}
          type="button"
          title={COLOR_NAMES[i]}
          aria-label={COLOR_NAMES[i]}
          aria-pressed={color === value}
          className="size-5 rounded-full border-0 p-0 aria-pressed:outline-2 aria-pressed:outline-offset-3 aria-pressed:outline-blue-500 sm:p-0"
          onClick={() => change(value)}
        >
          <span
            className="size-5 shrink-0 self-center rounded-full"
            style={{ backgroundColor: value }}
          />
        </Button>
      ))}
    </div>
  );
}
function download(blob: Blob, name: string) {
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
function distanceToSegment(p: Point, a: Point, b: Point) {
  const dx = b[0] - a[0],
    dy = b[1] - a[1];
  const t = Math.max(
    0,
    Math.min(1, ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy || 1))
  );
  return Math.hypot(p[0] - a[0] - t * dx, p[1] - a[1] - t * dy);
}

function preventMiddleButtonDefault(e: React.MouseEvent<HTMLDivElement>) {
  if (e.button === 1) e.preventDefault();
}

export default function Editor({
  catalog,
  initialPlan,
  back,
}: {
  catalog: Catalog;
  initialPlan: Plan;
  back: () => void;
}) {
  const [plan, setPlan] = useState(initialPlan);
  const [data, setData] = useState<MapData | null>(null);
  const [ready, setReady] = useState(false);
  const [tool, setTool] = useState<Tool>("select");
  const [allianceId, setAllianceId] = useState(initialPlan.alliances[0]?.id ?? "");
  const [routeId, setRouteId] = useState(initialPlan.routes[0]?.id ?? "");
  const [color, setColor] = useState(PALETTE[4]);
  const width = 2;
  const [text, setText] = useState("Rally here");
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [legs, setLegs] = useState<RouteLeg[]>([]);
  const [navigationReady, setNavigationReady] = useState(false);
  const [routing, setRouting] = useState(false);
  const [visibility, setVisibility] = useState<MapVisibility>({
    resources: true,
    allianceTags: true,
    caves: false,
    villages: false,
  });
  const [camera, setCamera] = useState<CameraStatus | null>(null);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [shareUrl, setShareUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [exporting, setExporting] = useState(false);
  const [exportingCoordinates, setExportingCoordinates] = useState(false);
  const host = useRef<HTMLDivElement>(null);
  const controller = useRef<LabController | null>(null);
  const worker = useRef<Worker | null>(null);
  const revision = useRef(0);
  const planRef = useRef(plan);
  const gesture = useRef<{
    x: number;
    y: number;
    lastX: number;
    lastY: number;
    start: Point;
    tool: Tool | "pan";
    moved: boolean;
    shift: boolean;
  } | null>(null);
  const route = plan.routes.find((r) => r.id === routeId);
  const commanderOptions = useCommanderOptions("en");
  const invalidAllianceTag = plan.alliances.some(
    (a) => !a.name.trim() || Array.from(a.name).length > 4
  );
  const alliance = plan.alliances.find((a) => a.id === allianceId);
  const summary =
    data && plan.mode === "territory" ? territorySummary(plan, catalog, data, allianceId) : null;

  useEffect(() => {
    planRef.current = plan;
  }, [plan]);

  useEffect(() => {
    controller.current?.setPlacementTool(tool, allianceId);
  }, [tool, allianceId]);

  useEffect(() => {
    const abort = new AbortController();
    loadMapData(catalog, initialPlan.mode, abort.signal)
      .then(setData)
      .catch((e) => {
        if (!abort.signal.aborted) setError(String(e));
      });
    return () => abort.abort();
  }, [catalog, initialPlan.mode]);

  useEffect(() => {
    if (!data || !host.current) return;
    let cancelled = false;
    const instance = new LabController(host.current, catalog, data, setCamera, setError);
    controller.current = instance;
    const webgl = new URLSearchParams(location.search).get("renderer") === "webgl";
    instance
      .start(webgl)
      .then(() => {
        if (cancelled) return;
        instance.update(planRef.current, new Set(), [], {
          resources: true,
          allianceTags: true,
          caves: false,
          villages: false,
        });
        setReady(true);
      })
      .catch((e) => {
        if (!cancelled) setError(`The map renderer could not start: ${String(e)}`);
      });
    return () => {
      cancelled = true;
      controller.current = null;
      instance.dispose();
    };
  }, [catalog, data]);

  useEffect(() => {
    const canvasHost = host.current;
    if (!canvasHost) return;
    // React wheel handlers are passive; this listener keeps zoom gestures from
    // also scrolling the page, including trackpad gestures at the zoom limits.
    const zoomMap = (event: WheelEvent) => {
      event.preventDefault();
      if (!ready) return;
      const rect = canvasHost.getBoundingClientRect();
      const unit =
        event.deltaMode === WheelEvent.DOM_DELTA_LINE
          ? 16
          : event.deltaMode === WheelEvent.DOM_DELTA_PAGE
            ? rect.height
            : 1;
      controller.current?.zoom(
        Math.exp(-event.deltaY * unit * 0.0015),
        event.clientX - rect.left,
        event.clientY - rect.top
      );
    };
    canvasHost.addEventListener("wheel", zoomMap, { passive: false });
    return () => canvasHost.removeEventListener("wheel", zoomMap);
  }, [ready]);

  useEffect(() => {
    if (!data || initialPlan.mode !== "baulur") return;
    const instance = new Worker(
      new URL("../../lib/territory-lab/navigation.worker.ts", import.meta.url),
      { type: "module" }
    );
    worker.current = instance;
    instance.onmessage = (e) => {
      if (e.data.type === "ready") setNavigationReady(true);
      else if (e.data.type === "routes" && e.data.revision === revision.current) {
        setLegs(e.data.legs);
        setRouting(false);
      } else if (e.data.type === "error") {
        setError(e.data.message);
        setRouting(false);
      }
    };
    instance.onerror = () => {
      setError("Route calculation failed. Reload the map to retry.");
      setRouting(false);
    };
    instance.postMessage({
      type: "init",
      bounds: catalog.bounds,
      bytes: data.forbiddenBytes,
      sites: data.sites,
    });
    return () => {
      instance.terminate();
      worker.current = null;
    };
  }, [catalog.bounds, data, initialPlan.mode]);

  useEffect(() => {
    if (!navigationReady || !worker.current) return;
    revision.current += 1;
    setRouting(true);
    worker.current.postMessage({ type: "routes", revision: revision.current, routes: plan.routes });
  }, [navigationReady, plan.routes]);

  useEffect(() => {
    if (!ready) return;
    try {
      controller.current?.update(plan, selected, legs, visibility);
    } catch (e) {
      setError(String(e));
    }
  }, [plan, ready, selected, legs, visibility]);

  function edit(next: Plan | ((p: Plan) => Plan)) {
    setPlan(next);
    setShareUrl("");
    setError("");
  }
  function updateRoute(update: (r: Route) => Route) {
    edit((p) => ({ ...p, routes: p.routes.map((r) => (r.id === routeId ? update(r) : r)) }));
  }
  function removeSelected() {
    edit((p) => ({
      ...p,
      buildings: p.buildings.filter((b) => !selected.has(b.id)),
      annotations: p.annotations.filter((a) => !selected.has(a.id)),
    }));
    setSelected(new Set());
  }
  function pick(x: number, y: number): string | undefined {
    const engine = controller.current;
    if (!engine) return;
    for (const a of [...plan.annotations].reverse()) {
      const p = a.points.map(([px, py]) => engine.screen(px, py));
      if (a.kind === "circle") {
        if (
          Math.abs(
            Math.hypot(x - p[0][0], y - p[0][1]) - Math.hypot(p[1][0] - p[0][0], p[1][1] - p[0][1])
          ) < 10
        )
          return a.id;
      } else if (p.length === 1) {
        if (Math.hypot(x - p[0][0], y - p[0][1]) < (a.kind === "text" ? 40 : 18)) return a.id;
      } else if (p.slice(1).some((b, i) => distanceToSegment([x, y], p[i], b) < 10)) return a.id;
    }
    return [...plan.buildings].reverse().find((b) => {
      if (b.kind === "flag" && (camera?.zoom ?? 1) <= 5) return false;
      const p = engine.screen(b.x, b.y);
      return Math.hypot(x - p[0], y - p[1]) < 18;
    })?.id;
  }
  function assignSite(site: Site) {
    if (!route) {
      setError("Select a route before assigning sites.");
      return;
    }
    if (site.kind === "pass") {
      if (!alliance) {
        setError("Create an alliance before assigning a pass.");
        return;
      }
      edit((p) => ({ ...p, routes: toggleRoutePass(p.routes, routeId, site.id, allianceId) }));
    } else {
      edit((p) => ({ ...p, routes: toggleRouteSite(p.routes, routeId, site.id) }));
    }
  }
  function click(x: number, y: number, shift: boolean) {
    const engine = controller.current;
    if (!engine || !data) return;
    const p = engine.world(x, y);
    if (
      p[0] < catalog.bounds[0] ||
      p[1] < catalog.bounds[1] ||
      p[0] > catalog.bounds[2] ||
      p[1] > catalog.bounds[3]
    )
      return;
    if (tool === "select") {
      const id = pick(x, y);
      if (id) {
        setSelected((previous) => updatePlannerSelection(previous, id, shift));
        return;
      }
      if (plan.mode === "baulur") {
        const site = engine.pickSite(x, y);
        if (site) assignSite(site);
      }
      setSelected(new Set());
    } else if (["flag", "fortress", "center-fortress", "horse"].includes(tool)) {
      if (!alliance) {
        setError("Create an alliance first.");
        return;
      }
      const building = {
        id: crypto.randomUUID(),
        kind: tool as BuildingKind,
        allianceId,
        x: Math.round(p[0] * 100) / 100,
        y: Math.round(p[1] * 100) / 100,
      };
      const problem = placementError(building, plan, catalog, data);
      if (problem) {
        setError(problem);
        return;
      }
      edit((previous) => ({ ...previous, buildings: [...previous.buildings, building] }));
      setMessage("");
    } else if (tool === "marker" || tool === "text") {
      if (plan.annotations.length >= 500) {
        setError("A plan supports up to 500 annotations.");
        return;
      }
      if (tool === "text" && plan.annotations.filter((a) => a.kind === "text").length >= 128) {
        setError("A plan supports up to 128 text labels.");
        return;
      }
      if (tool === "text" && !text.trim()) {
        setError("Enter the label text first.");
        return;
      }
      const annotation: Annotation = {
        id: crypto.randomUUID(),
        kind: tool,
        points: [p],
        color,
        width,
        ...(tool === "text" ? { text: text.trim() } : {}),
      };
      edit((previous) => ({ ...previous, annotations: [...previous.annotations, annotation] }));
    }
  }
  function pointerDown(e: React.PointerEvent<HTMLDivElement>) {
    preventMiddleButtonDefault(e);
    if (!ready || !controller.current || e.button > 1) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left,
      y = e.clientY - rect.top;
    gesture.current = {
      x,
      y,
      lastX: x,
      lastY: y,
      start: controller.current.world(x, y),
      tool: e.button === 1 ? "pan" : tool,
      moved: false,
      shift: e.shiftKey,
    };
    if (e.button === 0 && ["line", "arrow", "circle"].includes(tool))
      controller.current.beginDraft(tool, x, y, color, width);
    e.currentTarget.setPointerCapture(e.pointerId);
    controller.current.canvas.focus({ preventScroll: true });
  }
  function pointerMove(e: React.PointerEvent<HTMLDivElement>) {
    const g = gesture.current,
      engine = controller.current;
    if (!engine || !ready) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left,
      y = e.clientY - rect.top;
    if (["flag", "fortress", "center-fortress", "horse"].includes(tool))
      engine.previewPlacement(tool as BuildingKind, allianceId, x, y);
    if (!g) return;
    g.moved ||= Math.hypot(x - g.x, y - g.y) > 4;
    if (g.moved && (g.tool === "pan" || g.tool === "select")) engine.pan(x - g.lastX, y - g.lastY);
    if (g.moved && ["line", "arrow", "circle"].includes(g.tool)) engine.draftPoint(x, y);
    g.lastX = x;
    g.lastY = y;
  }
  function pointerUp() {
    const g = gesture.current,
      engine = controller.current;
    gesture.current = null;
    if (!g || !engine) return;
    engine.cancelDraft();
    if (!g.moved) {
      if (g.tool !== "pan") click(g.x, g.y, g.shift);
      return;
    }
    if (!["line", "arrow", "circle"].includes(g.tool)) return;
    if (plan.annotations.length >= 500) {
      setError("A plan supports up to 500 annotations.");
      return;
    }
    const end = engine.world(g.lastX, g.lastY);
    const points = [g.start, end].map(
      (p) =>
        [
          Math.max(catalog.bounds[0], Math.min(catalog.bounds[2], p[0])),
          Math.max(catalog.bounds[1], Math.min(catalog.bounds[3], p[1])),
        ] as Point
    );
    edit((p) => ({
      ...p,
      annotations: [
        ...p.annotations,
        {
          id: crypto.randomUUID(),
          kind: g.tool as "line" | "arrow" | "circle",
          points,
          color,
          width,
        },
      ],
    }));
  }
  async function share() {
    setBusy(true);
    setError("");
    try {
      const response = await fetch(`${API}/share`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(plan),
      });
      if (!response.ok)
        throw new Error("Could not share this plan. Check route commander names and try again.");
      const { id } = await response.json();
      const url = new URL("/territory-lab", "https://rokbattles.com");
      url.searchParams.set("share", id);
      setShareUrl(url.href);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }
  async function exportCoordinates() {
    if (!data) return;
    setExportingCoordinates(true);
    setError("");
    try {
      const { exportPlannerSpreadsheet } = await import("@/lib/territory-lab/export");
      await exportPlannerSpreadsheet(plan, data);
    } catch {
      setError("The spreadsheet could not be exported. Please try again.");
    } finally {
      setExportingCoordinates(false);
    }
  }
  const updateColor = (value: string) => {
    setColor(value);
    if (selected.size)
      edit((p) => ({
        ...p,
        annotations: p.annotations.map((a) => (selected.has(a.id) ? { ...a, color: value } : a)),
      }));
  };

  return (
    <div className="space-y-4">
      <div className="mb-8">
        <Link
          href="/territory-lab"
          className="inline-flex items-center gap-2 text-sm/6 text-zinc-500 dark:text-zinc-400"
          onClick={(event) => {
            if (
              event.button !== 0 ||
              event.metaKey ||
              event.ctrlKey ||
              event.shiftKey ||
              event.altKey
            )
              return;
            event.preventDefault();
            back();
          }}
        >
          <ChevronLeftIcon className="size-4 fill-zinc-400 dark:fill-zinc-500" />
          Maps
        </Link>
      </div>
      <header className="flex flex-wrap items-center gap-4">
        <Heading>{catalog.name}</Heading>
        <div className="ml-auto flex flex-wrap gap-2 max-[720px]:ml-0">
          <Button
            outline
            type="button"
            disabled={!ready || busy || invalidAllianceTag}
            onClick={share}
          >
            {busy ? "Sharing..." : "Share plan"}
          </Button>
          <Button
            outline
            type="button"
            disabled={!ready || exportingCoordinates}
            onClick={exportCoordinates}
          >
            {exportingCoordinates ? "Exporting..." : "Export coordinates"}
          </Button>
          <Button
            outline
            type="button"
            disabled={!ready || exporting}
            onClick={async () => {
              setExporting(true);
              try {
                const blob = await controller.current?.png();
                if (blob) download(blob, `${catalog.map}.png`);
              } catch (e) {
                setError(String(e));
              } finally {
                setExporting(false);
              }
            }}
          >
            {exporting ? "Saving PNG..." : "Save PNG"}
          </Button>
        </div>
      </header>
      {invalidAllianceTag && (
        <Text>Use a nonempty alliance tag of up to four characters before sharing.</Text>
      )}
      {shareUrl && (
        <div className="flex flex-wrap items-center gap-3 [&>span]:w-96 [&>span]:max-w-full">
          <Input
            id="share-link"
            aria-label="Share link"
            readOnly
            value={shareUrl}
            onFocus={(e) => e.target.select()}
          />
          <Button
            outline
            type="button"
            onClick={() =>
              navigator.clipboard
                .writeText(shareUrl)
                .then(() => setMessage("Link copied."))
                .catch(() => setMessage("Select and copy the link above."))
            }
          >
            Copy link
          </Button>
        </div>
      )}
      {!catalog.categories.includes("structures") && (
        <div
          role="status"
          className="rounded-md border border-orange-300/60 bg-orange-50 px-4 py-3 text-sm text-orange-950 dark:border-orange-300/20 dark:bg-orange-400/10 dark:text-orange-100"
        >
          Structure data is currently unavailable for this map.
        </div>
      )}
      {error && (
        <div
          role="alert"
          className="rounded-md border border-red-300/60 bg-red-50 px-4 py-3 text-sm text-red-950 dark:border-red-300/20 dark:bg-red-400/10 dark:text-red-100"
        >
          {error}
        </div>
      )}
      <div className="grid h-[clamp(38rem,76svh,56rem)] min-h-152 grid-cols-[16rem_minmax(0,1fr)] overflow-hidden rounded-lg border border-current/12 max-[720px]:h-auto max-[720px]:grid-cols-1">
        <aside
          className="overflow-y-auto border-r border-current/12 p-4 max-[720px]:max-h-96 max-[720px]:border-r-0 [&_p]:my-2 [&_section]:mb-4 [&_section]:border-b [&_section]:border-current/12 [&_section]:pb-4 [&_section>button]:mt-2"
          aria-label="Planning tools"
        >
          <section>
            <Subheading>Alliances</Subheading>
            <Field className="mt-3">
              <Label>Active alliance</Label>
              <Listbox<string>
                value={allianceId}
                onChange={(selectedId) => setAllianceId(selectedId)}
              >
                {plan.alliances.map((a) => (
                  <ListboxOption key={a.id} value={a.id}>
                    <ListboxLabel>{a.name}</ListboxLabel>
                  </ListboxOption>
                ))}
              </Listbox>
            </Field>
            {alliance && (
              <>
                <Field className="mt-3">
                  <Label>Alliance tag</Label>
                  <Input
                    value={alliance.name}
                    onChange={(e) =>
                      edit((p) => ({
                        ...p,
                        alliances: p.alliances.map((a) =>
                          a.id === allianceId
                            ? { ...a, name: Array.from(e.target.value).slice(0, 4).join("") }
                            : a
                        ),
                      }))
                    }
                  />
                </Field>
                <Palette
                  color={alliance.color}
                  change={(value) =>
                    edit((p) => ({
                      ...p,
                      alliances: p.alliances.map((a) =>
                        a.id === allianceId ? { ...a, color: value } : a
                      ),
                    }))
                  }
                />
              </>
            )}
            <Button
              outline
              type="button"
              disabled={plan.alliances.length >= 16}
              onClick={() => {
                const id = crypto.randomUUID();
                edit((p) => ({
                  ...p,
                  alliances: [
                    ...p.alliances,
                    {
                      id,
                      name: `RB${String(p.alliances.length + 1).padStart(2, "0")}`,
                      color: PALETTE[p.alliances.length % PALETTE.length],
                    },
                  ],
                }));
                setAllianceId(id);
              }}
            >
              Add alliance
            </Button>
          </section>
          {plan.mode === "territory" && (
            <section>
              <Subheading>Build territory</Subheading>
              <div className="mt-2 grid grid-cols-1 gap-1.5">
                {BUILDING_LABELS.filter(([kind]) => catalog.buildings[kind]).map(
                  ([kind, label]) => (
                    <Button
                      outline
                      type="button"
                      key={kind}
                      disabled={!ready}
                      aria-pressed={tool === kind}
                      className="aria-pressed:outline-2 aria-pressed:-outline-offset-2 aria-pressed:outline-blue-500"
                      onClick={() => setTool(kind)}
                    >
                      {label}
                    </Button>
                  )
                )}
              </div>
            </section>
          )}
          {plan.mode === "baulur" && (
            <section>
              <Subheading>Routes</Subheading>
              <Field className="mt-3">
                <Label>Active route</Label>
                <Listbox<string> value={routeId} onChange={(selectedId) => setRouteId(selectedId)}>
                  {plan.routes.map((r) => (
                    <ListboxOption key={r.id} value={r.id}>
                      <ListboxLabel>{r.name}</ListboxLabel>
                    </ListboxOption>
                  ))}
                </Listbox>
              </Field>
              {route && (
                <Field className="mt-3">
                  <Label>Commander</Label>
                  <Combobox
                    options={commanderOptions.map((value) => value.name)}
                    displayValue={(name) => name ?? ""}
                    placeholder="Choose a commander"
                    value={route.commander}
                    onChange={(value) =>
                      updateRoute((current) => ({ ...current, commander: value ?? "" }))
                    }
                  >
                    {(name) => (
                      <ComboboxOption value={name}>
                        <ComboboxLabel>{name}</ComboboxLabel>
                      </ComboboxOption>
                    )}
                  </Combobox>
                  {route.commander && (
                    <Button
                      plain
                      className="mt-1 text-xs"
                      aria-label="Clear commander"
                      onClick={() => updateRoute((current) => ({ ...current, commander: "" }))}
                    >
                      Clear
                    </Button>
                  )}
                </Field>
              )}
              {(!navigationReady || routing) && (
                <Text>
                  {navigationReady ? "Calculating routes..." : "Preparing terrain navigation..."}
                </Text>
              )}
              {legs.some((leg) => !leg.path) && (
                <Text role="status">
                  Some stops are disconnected. Assign a connecting pass or change the route.
                </Text>
              )}
            </section>
          )}
          {plan.mode === "territory" && (
            <section>
              <Subheading>Annotate</Subheading>
              <div className="mt-2 grid grid-cols-2 gap-1.5">
                {TOOL_LABELS.map(([id, label]) => (
                  <Button
                    outline
                    type="button"
                    key={id}
                    disabled={!ready}
                    aria-pressed={tool === id}
                    className="aria-pressed:outline-2 aria-pressed:-outline-offset-2 aria-pressed:outline-blue-500"
                    onClick={() => setTool(id)}
                  >
                    {label}
                  </Button>
                ))}
              </div>
              <Palette color={color} change={updateColor} />
              {tool === "text" && (
                <Field className="mt-3">
                  <Label>Label text</Label>
                  <Input maxLength={120} value={text} onChange={(e) => setText(e.target.value)} />
                </Field>
              )}
              <div className="my-2 flex flex-wrap items-center gap-2">
                <Button
                  outline
                  type="button"
                  disabled={!ready}
                  onClick={() =>
                    setSelected(new Set([...plan.annotations, ...plan.buildings].map((o) => o.id)))
                  }
                >
                  Select all
                </Button>
                <Button outline type="button" disabled={!selected.size} onClick={removeSelected}>
                  Delete ({selected.size})
                </Button>
              </div>
            </section>
          )}
          <CheckboxGroup aria-label="Map visibility">
            {(
              [
                ["allianceTags", "Show alliance tags", true],
                ["resources", "Show resource points", plan.mode === "territory"],
                [
                  "caves",
                  "Show caves",
                  plan.mode === "territory" && catalog.categories.includes("caves"),
                ],
                [
                  "villages",
                  "Show villages",
                  plan.mode === "territory" && catalog.categories.includes("villages"),
                ],
              ] as const
            )
              .filter(([, , available]) => available)
              .map(([key, label]) => (
                <CheckboxField key={key}>
                  <Checkbox
                    checked={visibility[key]}
                    onChange={(checked) =>
                      setVisibility((previous) => ({ ...previous, [key]: checked }))
                    }
                  />
                  <Label>{label}</Label>
                </CheckboxField>
              ))}
          </CheckboxGroup>
        </aside>
        <section
          className="relative min-w-0 overflow-hidden bg-zinc-800 max-[720px]:h-[65svh] max-[720px]:min-h-96"
          aria-label="Map"
        >
          <div
            className={`size-full touch-none [&_canvas]:block [&_canvas]:size-full [&_canvas]:outline-none ${tool === "select" ? "cursor-grab active:cursor-grabbing" : "cursor-crosshair"}`}
            role="application"
            aria-label="Interactive planning map"
            ref={host}
            onPointerDown={pointerDown}
            onMouseDown={preventMiddleButtonDefault}
            onAuxClick={preventMiddleButtonDefault}
            onPointerMove={pointerMove}
            onPointerUp={pointerUp}
            onPointerLeave={() => controller.current?.clearPlacement()}
            onPointerCancel={() => {
              gesture.current = null;
              controller.current?.cancelDraft();
            }}
            onKeyDown={(e) => {
              if (plan.mode !== "territory") return;
              if (e.key === "Delete" || e.key === "Backspace") {
                e.preventDefault();
                removeSelected();
              } else if (e.key === "Escape") setSelected(new Set());
              else if ((e.ctrlKey || e.metaKey) && e.key === "a") {
                e.preventDefault();
                setSelected(new Set([...plan.annotations, ...plan.buildings].map((o) => o.id)));
              }
            }}
          />
          <div
            className="pointer-events-none absolute bottom-[0.8rem] left-[0.8rem] max-w-[calc(100%-1.6rem)] rounded-sm bg-zinc-900/90 px-[0.65rem] py-[0.4rem] text-xs text-zinc-300"
            role="status"
          >
            {ready
              ? `${camera?.backend} · ${camera?.zoom.toFixed(1)}×`
              : "Loading terrain and map artwork..."}
          </div>
        </section>
      </div>
      <Text className="mt-2 text-xs/5">
        {plan.mode === "baulur"
          ? "Drag to pan · Scroll to zoom · Click camps, keeps, and passes to assign them"
          : "Drag with Select to pan · Scroll to zoom · Shift-click to select multiple items · Delete to remove · Ctrl/Cmd+A to select all"}
      </Text>
      {message && <Text role="status">{message}</Text>}
      {data && (
        <section aria-label="Plan data" className="space-y-6">
          <Field className="max-w-xs">
            <Label>Active alliance</Label>
            <Listbox<string>
              value={allianceId}
              onChange={(selectedId) => setAllianceId(selectedId)}
            >
              {plan.alliances.map((value) => (
                <ListboxOption key={value.id} value={value.id}>
                  <ListboxLabel>{value.name}</ListboxLabel>
                </ListboxOption>
              ))}
            </Listbox>
          </Field>
          {summary && (
            <TerritoryDetails
              plan={plan}
              catalog={catalog}
              data={data}
              allianceId={allianceId}
              summary={summary}
              onLocate={(building) => {
                setSelected(new Set([building.id]));
                controller.current?.locate(building.x, building.y);
                host.current?.scrollIntoView({ block: "center", behavior: "smooth" });
              }}
            />
          )}
          {plan.mode === "baulur" && (
            <BaulurDetails
              plan={plan}
              data={data}
              onUpdate={(id, update) =>
                edit((previous) => ({
                  ...previous,
                  routes: previous.routes.map((value) => (value.id === id ? update(value) : value)),
                }))
              }
            />
          )}
        </section>
      )}
    </div>
  );
}

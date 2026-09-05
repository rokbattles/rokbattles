"use client";

import {
  ClipboardDocumentIcon,
  Cog6ToothIcon,
  CursorArrowRaysIcon,
  PencilIcon,
} from "@heroicons/react/20/solid";
import { useExtracted } from "next-intl";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { BuildingIcon } from "@/components/territory-planner/building-icon";
import { BuildingMetric } from "@/components/territory-planner/building-metric";
import {
  PlannerCanvas,
  type PlannerCanvasLocationRequest,
} from "@/components/territory-planner/planner-canvas";
import { PlannerSettingsDrawer } from "@/components/territory-planner/planner-settings-drawer";
import { ResourceMetric } from "@/components/territory-planner/resource-metric";
import { TerritoryBreakdown } from "@/components/territory-planner/territory-breakdown";
import { TerritoryExportButton } from "@/components/territory-planner/territory-export-button";
import { useCompactNumberFormatter } from "@/components/territory-planner/use-compact-number-formatter";
import { useTerritoryPlannerLabels } from "@/components/territory-planner/use-territory-planner-labels";
import { Button } from "@/components/ui/button";
import { Heading, Subheading } from "@/components/ui/heading";
import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/listbox";
import {
  buildingCostBreakdown,
  buildingCostProgress,
  calculateCostTotals,
} from "@/lib/territory/costs";
import { TerritoryDataSource, type WorldBounds } from "@/lib/territory/data-source";
import { decodePlan, encodePlan, normalizePlannerDocument } from "@/lib/territory/document";
import {
  alignBuildingPoint,
  boundaryCollision,
  buildingRules,
  buildTerritoryState,
  calculateResourceProduction,
  countCoveredResources,
  hasRequiredTerritoryAvailability,
  isBuildingKindAvailable,
  isConnectedToAlliance,
  isProvinceRestricted,
  mapStructureCollision,
  plannedBuildingCollision,
} from "@/lib/territory/geometry";
import { LOST_KINGDOM_TERRITORY_COLORS } from "@/lib/territory/presentation";
import { deleteSelectedPlannerItems, updatePlannerSelection } from "@/lib/territory/selection";
import type {
  Alliance,
  BuildingCostSchedule,
  BuildingKind,
  DrawingPoint,
  MapStructure,
  PlannedBuilding,
  PlannerDocument,
  PlannerTool,
  ResourceKind,
  ResourcePoint,
  TerritoryMapIndexRow,
} from "@/lib/territory/types";

const STORAGE_KEY = "territory-planner:v1";

const RESOURCE_KINDS: ResourceKind[] = ["food", "wood", "stone", "coin", "crystal"];
const EMPTY_COST_SCHEDULE: BuildingCostSchedule = {};
const EMPTY_BUILDING_CONFIGS = {};
const ZERO_PRODUCTION_RATES: Record<ResourceKind, number> = {
  food: 0,
  wood: 0,
  stone: 0,
  coin: 0,
  crystal: 0,
};
function createId(prefix: string): string {
  return `${prefix}-${globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`}`;
}

function initialDocument(maps: TerritoryMapIndexRow[], allianceName: string): PlannerDocument {
  const alliance = {
    id: "alliance-1",
    name: allianceName,
    color: LOST_KINGDOM_TERRITORY_COLORS[0].value,
  };
  return {
    version: 2,
    mapSlug: maps[0]?.slug ?? "s14-tides-of-war",
    activeAllianceId: alliance.id,
    alliances: [alliance],
    buildings: [],
    drawings: [],
  };
}

function loadedResourcesAtVersion(
  dataSource: TerritoryDataSource | null,
  dataVersion: number
): ResourcePoint[] {
  if (!dataSource || dataVersion < 0) return [];
  return dataSource.allLoadedResources();
}

function loadedStructuresAtVersion(
  dataSource: TerritoryDataSource | null,
  dataVersion: number
): MapStructure[] {
  if (!dataSource || dataVersion < 0) return [];
  return dataSource.allLoadedStructures();
}

export function TerritoryPlanner({ maps }: { maps: TerritoryMapIndexRow[] }) {
  const t = useExtracted();
  const numberFormatter = useCompactNumberFormatter();
  const { mapLabel, resourceLabel, toolLabel } = useTerritoryPlannerLabels();
  const [document, setDocument] = useState<PlannerDocument>(() => initialDocument(maps, "ROKB"));
  const [loadedDataSource, setLoadedDataSource] = useState<TerritoryDataSource | null>(null);
  const [dataVersion, setDataVersion] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [tool, setTool] = useState<PlannerTool>("flag");
  const [selectedItemIds, setSelectedItemIds] = useState<ReadonlySet<string>>(() => new Set());
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [showBoundary, setShowBoundary] = useState(true);
  const [showResources, setShowResources] = useState(true);
  const [showVillages, setShowVillages] = useState(false);
  const [showCaves, setShowCaves] = useState(false);
  const [shareStatus, setShareStatus] = useState("");
  const [locationRequest, setLocationRequest] = useState<PlannerCanvasLocationRequest | null>(null);
  const sourceRef = useRef<TerritoryDataSource | null>(null);
  const viewportRef = useRef<WorldBounds | null>(null);
  const mapSectionRef = useRef<HTMLElement>(null);
  const locationSequenceRef = useRef(0);
  const loadGenerationRef = useRef(0);
  const hydratedRef = useRef(false);
  const availableMapSlugs = useMemo(() => new Set(maps.map((map) => map.slug)), [maps]);

  const map = maps.find((candidate) => candidate.slug === document.mapSlug) ?? maps[0];
  const dataSource = loadedDataSource?.config.slug === map?.slug ? loadedDataSource : null;
  const mapLoadError = t("The map data could not be loaded.");
  const activeAlliance =
    document.alliances.find((alliance) => alliance.id === document.activeAllianceId) ??
    document.alliances[0];
  const structures = useMemo(
    () => loadedStructuresAtVersion(dataSource, dataVersion),
    [dataSource, dataVersion]
  );
  const territoryState = useMemo(
    () => buildTerritoryState(document.buildings, structures),
    [document.buildings, structures]
  );
  const territoryOwnership = territoryState.ownership;

  const commit = useCallback((mutate: (document: PlannerDocument) => PlannerDocument) => {
    setDocument((current) => mutate(current));
  }, []);

  useEffect(() => {
    if (hydratedRef.current) return;
    hydratedRef.current = true;
    try {
      const hashValue = new URLSearchParams(window.location.hash.slice(1)).get("plan");
      const fromHash = hashValue ? decodePlan(hashValue) : null;
      const stored = localStorage.getItem(STORAGE_KEY);
      const fromStorage = stored ? JSON.parse(stored) : null;
      const restored =
        normalizePlannerDocument(fromHash, availableMapSlugs) ??
        normalizePlannerDocument(fromStorage, availableMapSlugs);
      if (restored) setDocument(restored);
    } catch {
      // Invalid local state falls back to a new plan.
    }
  }, [availableMapSlugs]);

  useEffect(() => {
    if (!hydratedRef.current) return;
    const save = () => {
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(document));
      } catch {
        // Storage can be unavailable or full. The in-memory planner remains usable.
      }
    };
    const idle = window.requestIdleCallback?.(save, { timeout: 1200 });
    const timeout = idle === undefined ? window.setTimeout(save, 250) : undefined;
    return () => {
      if (idle !== undefined) window.cancelIdleCallback?.(idle);
      if (timeout !== undefined) window.clearTimeout(timeout);
    };
  }, [document]);

  useEffect(() => {
    if (!map) return;
    const generation = ++loadGenerationRef.current;
    const controller = new AbortController();
    setLoading(true);
    setError("");
    setLoadedDataSource(null);
    sourceRef.current = null;
    TerritoryDataSource.load(map.slug, controller.signal)
      .then(async (source) => {
        if (generation !== loadGenerationRef.current) return;
        sourceRef.current = source;
        setLoadedDataSource(source);
        const viewport = viewportRef.current;
        if (viewport) await source.ensureBounds(viewport, controller.signal);
        if (generation === loadGenerationRef.current) setDataVersion((value) => value + 1);
      })
      .catch(() => {
        if (controller.signal.aborted || generation !== loadGenerationRef.current) return;
        setError(mapLoadError);
      })
      .finally(() => {
        if (generation === loadGenerationRef.current) setLoading(false);
      });
    return () => controller.abort();
  }, [map, mapLoadError]);

  useEffect(() => {
    if (!dataSource) return;
    const controller = new AbortController();
    const chunkCount = dataSource.chunks.size;
    dataSource
      .ensureBuildings(document.buildings, controller.signal)
      .then(() => {
        if (dataSource.chunks.size !== chunkCount) setDataVersion((value) => value + 1);
      })
      .catch(() => undefined);
    return () => controller.abort();
  }, [dataSource, document.buildings]);

  const handleViewportChange = useCallback((bounds: WorldBounds) => {
    viewportRef.current = bounds;
    const source = sourceRef.current;
    if (!source) return;
    const generation = loadGenerationRef.current;
    const chunkCount = source.chunks.size;
    source
      .ensureBounds(bounds)
      .then(() => {
        if (generation === loadGenerationRef.current && source.chunks.size !== chunkCount) {
          setDataVersion((value) => value + 1);
        }
      })
      .catch(() => undefined);
  }, []);

  const isPlacementLegal = useCallback(
    (kind: BuildingKind, rawX: number, rawY: number) => {
      const source = sourceRef.current;
      if (!source) return false;
      if (!isBuildingKindAvailable(kind, source.config.ruleset, source.config.supportsHorse)) {
        return false;
      }
      const progress = buildingCostProgress(
        kind,
        document.buildings,
        source.costs,
        activeAlliance.id,
        source.buildings
      );
      if (progress.built >= progress.limit) return false;
      const { x, y } = alignBuildingPoint(rawX, rawY);
      const imageBounds = source.config.imageBounds;
      if (
        x < imageBounds.minX ||
        x >= imageBounds.maxX ||
        y < imageBounds.minY ||
        y >= imageBounds.maxY
      ) {
        return false;
      }
      if (isProvinceRestricted(source.provinceGrid, kind, x, y)) return false;
      const clearance = buildingRules[kind].baseClearance;
      const area = {
        minX: x - clearance,
        minY: y - clearance,
        maxX: x + clearance,
        maxY: y + clearance,
      };
      const structureBuffer = Math.max(1, source.config.spatial.chunkBuffer);
      const coordinates = source.chunkCoordinates(area, structureBuffer);
      const chunks = source.chunksWithin(area, structureBuffer);
      if (chunks.length < coordinates.length) return false;
      const instances = chunks.flatMap((chunk) => chunk.instances);
      if (boundaryCollision(x, y, clearance, source.definitions, instances)) return false;
      const candidate: PlannedBuilding = {
        id: "candidate",
        allianceId: activeAlliance.id,
        kind,
        x,
        y,
      };
      if (
        mapStructureCollision(
          candidate,
          chunks.flatMap((chunk) => chunk.structures)
        )
      ) {
        return false;
      }
      if (plannedBuildingCollision(candidate, document.buildings)) return false;
      if (!hasRequiredTerritoryAvailability(candidate, territoryOwnership)) return false;
      return isConnectedToAlliance(candidate, territoryOwnership);
    },
    [activeAlliance.id, document.buildings, territoryOwnership]
  );

  const place = useCallback(
    async (kind: BuildingKind, rawX: number, rawY: number) => {
      const source = sourceRef.current;
      if (!source) return;
      const { x, y } = alignBuildingPoint(rawX, rawY);
      const clearance = buildingRules[kind].baseClearance;
      try {
        await source.ensureBounds(
          {
            minX: x - clearance,
            minY: y - clearance,
            maxX: x + clearance,
            maxY: y + clearance,
          },
          undefined,
          Math.max(1, source.config.spatial.chunkBuffer)
        );
      } catch {
        return;
      }
      setDataVersion((value) => value + 1);
      if (!isPlacementLegal(kind, x, y)) return;
      const building: PlannedBuilding = {
        id: createId("building"),
        allianceId: activeAlliance.id,
        kind,
        x,
        y,
      };
      commit((current) => ({ ...current, buildings: [...current.buildings, building] }));
      setSelectedItemIds(new Set([building.id]));
    },
    [activeAlliance.id, commit, isPlacementLegal]
  );

  const addAlliance = useCallback(
    (name: string, color: string) => {
      const alliance: Alliance = {
        id: createId("alliance"),
        name,
        color,
      };
      commit((current) => ({
        ...current,
        activeAllianceId: alliance.id,
        alliances: [...current.alliances, alliance],
      }));
      setSelectedItemIds(new Set());
    },
    [commit]
  );

  const changeActiveAlliance = useCallback(
    (allianceId: string) => {
      commit((current) => ({ ...current, activeAllianceId: allianceId }));
      setSelectedItemIds(new Set());
    },
    [commit]
  );

  const changeAlliance = useCallback(
    (allianceId: string, change: Partial<Pick<Alliance, "name" | "color">>) => {
      commit((current) => ({
        ...current,
        alliances: current.alliances.map((alliance) =>
          alliance.id === allianceId ? { ...alliance, ...change } : alliance
        ),
      }));
    },
    [commit]
  );

  const deleteAlliance = useCallback(
    (allianceId: string) => {
      commit((current) => {
        if (current.alliances.length <= 1) return current;
        const alliances = current.alliances.filter((alliance) => alliance.id !== allianceId);
        return {
          ...current,
          activeAllianceId:
            current.activeAllianceId === allianceId
              ? (alliances[0]?.id ?? current.activeAllianceId)
              : current.activeAllianceId,
          alliances,
          buildings: current.buildings.filter((building) => building.allianceId !== allianceId),
          drawings: current.drawings.filter((drawing) => drawing.allianceId !== allianceId),
        };
      });
      setSelectedItemIds(new Set());
    },
    [commit]
  );

  const addDrawing = useCallback(
    (allianceId: string, points: DrawingPoint[]) => {
      if (points.length < 2) return;
      const drawing = {
        id: createId("drawing"),
        allianceId,
        points,
      };
      commit((current) => ({
        ...current,
        drawings: [...current.drawings, drawing],
      }));
      setSelectedItemIds(new Set([drawing.id]));
    },
    [commit]
  );

  const selectItem = useCallback((itemId: string | null, additive: boolean) => {
    setSelectedItemIds((current) => updatePlannerSelection(current, itemId, additive));
  }, []);

  const deleteSelection = useCallback(() => {
    if (selectedItemIds.size === 0) return;
    commit((current) => deleteSelectedPlannerItems(current, selectedItemIds));
    setSelectedItemIds(new Set());
  }, [commit, selectedItemIds]);

  const locateBuilding = useCallback((building: PlannedBuilding) => {
    setTool("select");
    setSelectedItemIds(new Set([building.id]));
    setLocationRequest({
      requestId: ++locationSequenceRef.current,
      x: building.x,
      y: building.y,
    });
    mapSectionRef.current?.scrollIntoView({
      behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth",
      block: "start",
    });
  }, []);

  const closeSettings = useCallback(() => setSettingsOpen(false), []);

  const resources = useMemo(
    () => loadedResourcesAtVersion(dataSource, dataVersion),
    [dataSource, dataVersion]
  );
  const coveredResources = useMemo(
    () => countCoveredResources(territoryOwnership, resources, activeAlliance.id),
    [activeAlliance.id, resources, territoryOwnership]
  );
  const productionRates = dataSource?.productionRates ?? ZERO_PRODUCTION_RATES;
  const resourceProduction = useMemo(
    () => calculateResourceProduction(coveredResources, productionRates),
    [coveredResources, productionRates]
  );
  const costSchedule = dataSource?.costs ?? EMPTY_COST_SCHEDULE;
  const buildingConfigs = dataSource?.buildings ?? EMPTY_BUILDING_CONFIGS;
  const costProgress = useMemo(
    () =>
      Object.fromEntries(
        (["flag", "mainFortress", "subFortress", "horse"] as BuildingKind[]).map((kind) => [
          kind,
          buildingCostProgress(
            kind,
            document.buildings,
            costSchedule,
            activeAlliance.id,
            buildingConfigs
          ),
        ])
      ) as Record<BuildingKind, ReturnType<typeof buildingCostProgress>>,
    [activeAlliance.id, buildingConfigs, costSchedule, document.buildings]
  );
  const costBreakdown = useMemo(
    () => buildingCostBreakdown(document.buildings, costSchedule, activeAlliance.id),
    [activeAlliance.id, costSchedule, document.buildings]
  );
  const totals = useMemo(
    () => calculateCostTotals(document.buildings, costSchedule, activeAlliance.id),
    [activeAlliance.id, costSchedule, document.buildings]
  );

  const share = useCallback(async () => {
    const value = encodePlan(document);
    const url = new URL(window.location.href);
    url.hash = new URLSearchParams({ plan: value }).toString();
    window.history.replaceState(null, "", url);
    try {
      await navigator.clipboard.writeText(url.toString());
      setShareStatus(t("Shareable plan URL copied to the clipboard."));
    } catch {
      setShareStatus(t("The shareable plan is now in the address bar."));
    }
  }, [document, t]);

  if (!map) return <p>{t("No maps are available.")}</p>;
  const toolOptions: PlannerTool[] = ["select", "draw", "flag"];
  if (map.ruleset === "home") toolOptions.push("mainFortress");
  toolOptions.push("subFortress");
  if (map.supportsHorse) toolOptions.push("horse");

  return (
    <div className="space-y-5">
      <header className="space-y-4">
        <Heading>{t("Territory Planner")}</Heading>
        <div className="flex min-w-0 flex-col gap-2 sm:flex-row">
          <Listbox<string>
            aria-label={t("Map")}
            className="min-w-0 sm:w-72 sm:flex-none"
            value={document.mapSlug}
            onChange={(mapSlug) => {
              commit((current) => ({ ...current, mapSlug, buildings: [], drawings: [] }));
              setSelectedItemIds(new Set());
              setLocationRequest(null);
            }}
          >
            {maps.map((option) => (
              <ListboxOption key={option.slug} value={option.slug}>
                <ListboxLabel>{mapLabel(option)}</ListboxLabel>
              </ListboxOption>
            ))}
          </Listbox>
          <Button className="rounded-md" onClick={share}>
            <ClipboardDocumentIcon /> {t("Copy URL")}
          </Button>
          <TerritoryExportButton document={document} />
          <span aria-live="polite" className="sr-only" role="status">
            {shareStatus}
          </span>
        </div>
      </header>

      <div className="flex flex-wrap items-center justify-between gap-3">
        <div aria-label={t("Building tools")} className="flex flex-wrap gap-2" role="toolbar">
          {toolOptions.map((kind) => (
            <Button
              aria-pressed={tool === kind}
              className="rounded-md"
              color={(tool === kind ? "blue" : "light") as "blue" | "light"}
              key={kind}
              onClick={() => setTool(kind)}
            >
              {kind === "select" ? (
                <CursorArrowRaysIcon />
              ) : kind === "draw" ? (
                <PencilIcon />
              ) : (
                <BuildingIcon className="size-6" kind={kind} />
              )}
              {toolLabel(kind)}
            </Button>
          ))}
        </div>

        <div className="order-first flex w-full min-w-0 gap-2 lg:order-last lg:ms-auto lg:w-auto">
          <Listbox<string>
            aria-label={t("Alliance")}
            className="min-w-0 flex-1 lg:w-56 lg:flex-none"
            value={activeAlliance.id}
            onChange={changeActiveAlliance}
          >
            {document.alliances.map((alliance) => (
              <ListboxOption key={alliance.id} value={alliance.id}>
                <ListboxLabel>{alliance.name}</ListboxLabel>
              </ListboxOption>
            ))}
          </Listbox>
          <Button
            aria-label={t("Planner settings")}
            className="rounded-md"
            outline
            onClick={() => setSettingsOpen(true)}
          >
            <Cog6ToothIcon />
          </Button>
        </div>
      </div>

      <div className="space-y-2">
        <section
          className="relative h-[clamp(32rem,70svh,46rem)] scroll-mt-20 overflow-hidden rounded-md ring-1 ring-zinc-950/10 dark:ring-white/10"
          ref={mapSectionRef}
        >
          {dataSource ? (
            <PlannerCanvas
              activeAllianceId={activeAlliance.id}
              alliances={document.alliances}
              buildings={document.buildings}
              dataSource={dataSource}
              dataVersion={dataVersion}
              drawings={document.drawings}
              isPlacementLegal={isPlacementLegal}
              key={dataSource.config.slug}
              locationRequest={locationRequest}
              onDeleteSelection={deleteSelection}
              onDraw={addDrawing}
              onPlace={place}
              onSelect={selectItem}
              onViewportChange={handleViewportChange}
              selectedItemIds={selectedItemIds}
              showBoundary={showBoundary}
              showCaves={showCaves}
              showResources={showResources}
              showVillages={showVillages}
              structures={structures}
              territoryState={territoryState}
              tool={tool}
            />
          ) : (
            <div className="flex size-full min-h-[38rem] items-center justify-center bg-zinc-100 text-sm text-zinc-600 dark:bg-zinc-800 dark:text-zinc-300">
              {error ||
                (loading ? t("Loading map geometry...") : t("Map geometry is unavailable."))}
            </div>
          )}
        </section>
        <p className="text-xs text-zinc-500 dark:text-zinc-400">
          {t("Scroll to zoom")} &middot; {t("Drag to pan")} &middot;{" "}
          {t("Shift + click to multi-select")} &middot;{" "}
          {t("Delete / Backspace to delete selected items")}
        </p>
      </div>

      <div className="grid min-h-0 gap-8 overflow-hidden xl:grid-cols-[minmax(0,1.35fr)_minmax(20rem,1fr)] xl:divide-x xl:divide-zinc-950/10 dark:xl:divide-white/10">
        <TerritoryBreakdown
          entries={costBreakdown}
          onLocate={locateBuilding}
          resources={resources}
          territoryOwnership={territoryOwnership}
        />

        <section aria-labelledby="plan-total-heading" className="min-w-0 xl:pl-8">
          <Subheading id="plan-total-heading">{t("Territory limits & costs")}</Subheading>
          <dl className="mt-4 grid grid-cols-2 gap-x-5 gap-y-2.5 text-sm">
            {map.ruleset === "home" ? (
              <BuildingMetric
                kind="mainFortress"
                label={t("Center Fortress")}
                value={`${costProgress.mainFortress.built} / ${costProgress.mainFortress.limit}`}
              />
            ) : null}
            <BuildingMetric
              kind="subFortress"
              label={t("Fortresses")}
              value={`${costProgress.subFortress.built} / ${costProgress.subFortress.limit}`}
            />
            {map.supportsHorse ? (
              <BuildingMetric
                kind="horse"
                label={t("Horse")}
                value={`${costProgress.horse.built} / ${costProgress.horse.limit}`}
              />
            ) : null}
            <BuildingMetric
              kind="flag"
              label={t("Flags")}
              value={`${costProgress.flag.built} / ${costProgress.flag.limit}`}
            />
            <ResourceMetric
              icon="credits"
              label={t("Alliance credits")}
              value={numberFormatter.format(totals.credits)}
            />
            <ResourceMetric
              icon="crystal"
              label={t("Crystal")}
              value={numberFormatter.format(totals.crystal)}
            />
            <ResourceMetric
              icon="food"
              label={t("Food")}
              value={numberFormatter.format(totals.food)}
            />
            <ResourceMetric
              icon="wood"
              label={t("Wood")}
              value={numberFormatter.format(totals.wood)}
            />
            <ResourceMetric
              icon="stone"
              label={t("Stone")}
              value={numberFormatter.format(totals.stone)}
            />
            <ResourceMetric
              icon="coin"
              label={t("Gold")}
              value={numberFormatter.format(totals.gold)}
            />
          </dl>
          {totals.unknown ? (
            <p className="mt-3 text-xs/5 text-amber-700 dark:text-amber-400">
              {t(
                "{count, plural, one {# planned building has} other {# planned buildings have}} no construction-cost entry.",
                { count: totals.unknown }
              )}
            </p>
          ) : null}

          <div className="mt-6 border-zinc-950/10 border-t pt-6 dark:border-white/10">
            <Subheading>{t("Territory RSS production")}</Subheading>
            <dl className="mt-4 grid grid-cols-2 gap-x-5 gap-y-2.5 text-sm">
              {RESOURCE_KINDS.map((kind) => {
                const production = resourceProduction[kind];
                return (
                  <ResourceMetric
                    icon={kind}
                    key={kind}
                    label={resourceLabel(kind)}
                    value={t("{amount}/h", { amount: numberFormatter.format(production) })}
                  />
                );
              })}
            </dl>
          </div>
        </section>
      </div>

      <PlannerSettingsDrawer
        alliances={document.alliances}
        open={settingsOpen}
        showBoundary={showBoundary}
        showCaves={showCaves}
        showResources={showResources}
        showVillages={showVillages}
        onAddAlliance={addAlliance}
        onChangeAlliance={changeAlliance}
        onClose={closeSettings}
        onDeleteAlliance={deleteAlliance}
        onShowBoundaryChange={setShowBoundary}
        onShowCavesChange={setShowCaves}
        onShowResourcesChange={setShowResources}
        onShowVillagesChange={setShowVillages}
      />
    </div>
  );
}

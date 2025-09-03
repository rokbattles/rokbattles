import { resolveNames } from "@/actions/datasets";
import { cn } from "@/lib/cn";
import { roman } from "@/lib/roman";
import type { ParticipantInfo } from "@/lib/types/reports";

function SectionHeading({ title }: { title: string }) {
  return <h4 className="text-base font-semibold text-zinc-100">{title}</h4>;
}

function Badge({
  children,
  color,
  size = "sm",
}: {
  children: React.ReactNode;
  color?: "gray" | "blue" | "amber";
  size?: "sm" | "md";
}) {
  return (
    <span
      className={cn(
        "inline-flex items-center rounded-md font-semibold ring-1 ring-inset",
        size === "md" ? "px-2 py-1 text-xs" : "px-1.5 py-0.5 text-[10px]",
        color === "blue"
          ? "ring-blue-400/50"
          : color === "amber"
            ? "ring-amber-400/50"
            : "ring-white/20",
        color === "gray" || !color ? "text-zinc-100" : "text-zinc-100"
      )}
    >
      {children}
    </span>
  );
}

type EquipToken = { slot: number; id: number; craft?: number; attr?: number };

function parseEquipment(raw?: string): EquipToken[] {
  if (!raw) return [];
  const inner = raw.trim().replace(/^[{\s]*|[}\s]*$/g, "");
  if (!inner) return [];
  const parts = inner.split(",");
  const out: EquipToken[] = [];
  for (const p of parts) {
    const segs = p.split(":");
    if (segs.length < 2) continue;
    const slot = Number(segs[0]);
    const idCraft = segs[1] ?? "";
    const attrStr = segs[2];
    const idStr = idCraft.split("_")[0] ?? idCraft;
    const craftStr = idCraft.includes("_") ? idCraft.split("_")[1] : undefined;
    const id = Number(idStr);
    const craft = craftStr ? Number(craftStr) : undefined;
    const attr = attrStr !== undefined ? Number(attrStr) : undefined;
    if (Number.isFinite(slot) && Number.isFinite(id)) {
      out.push({
        slot,
        id,
        craft: Number.isFinite(craft) ? craft : undefined,
        attr: Number.isFinite(attr) ? attr : undefined,
      });
    }
  }
  return out.sort((a, b) => a.slot - b.slot);
}

function parseSemicolonIds(raw?: string): number[] {
  if (!raw) return [];
  return raw
    .split(";")
    .map((s) => Number(s))
    .filter((n) => Number.isFinite(n) && n !== -1);
}

function parseArmamentPairs(raw?: string): Array<{ id: number; value: number }> {
  if (!raw) return [];
  const out: Array<{ id: number; value: number }> = [];
  for (const token of raw.split(";")) {
    const [idStr, valStr] = token.split("_");
    const id = Number(idStr);
    const value = Number(valStr);
    if (Number.isFinite(id)) out.push({ id, value: Number.isFinite(value) ? value : 0 });
  }
  return out;
}

function deriveTierAndST(attr?: number): { tier?: number; isST: boolean } {
  if (!Number.isFinite(attr) || attr === undefined) return { isST: false };
  const n = Number(attr);
  const isST = n >= 10;
  const base = isST ? n % 10 : n;
  const tier = Math.max(0, Math.min(5, base));
  return { tier, isST };
}

function getInscriptionRarity(id: number): "common" | "rare" | "special" {
  if (id >= 1000) {
    const last = id % 10;
    if (last === 1) return "special";
    if (last === 2) return "rare";
  }
  return "common";
}

export default async function ParticipantGear({
  participant,
  locale = "en",
}: {
  participant?: ParticipantInfo;
  locale?: "en" | "es" | "kr";
}) {
  if (!participant) return null;

  const equip = parseEquipment(participant.equipment);
  const equipIdsUnique = Array.from(new Set(equip.map((e) => e.id))).map(String);
  const equipNameMap =
    equipIdsUnique.length > 0 ? await resolveNames("equipment", equipIdsUnique, locale) : {};

  const inscriptionIds = parseSemicolonIds(participant.inscriptions);
  const inscriptionNameMap =
    inscriptionIds.length > 0
      ? await resolveNames("inscriptions", inscriptionIds.map(String), locale)
      : {};

  const armPairs = parseArmamentPairs(participant.armament_buffs);
  const agg: Record<number, number> = {};
  for (const { id, value } of armPairs) {
    agg[id] = (agg[id] ?? 0) + (Number.isFinite(value) ? value : 0);
  }
  const armamentIds = Object.keys(agg).map((k) => Number(k));
  const armamentNameMap =
    armamentIds.length > 0 ? await resolveNames("armaments", armamentIds.map(String), locale) : {};

  return (
    <div className="space-y-4">
      <div>
        <SectionHeading title="Equipment" />
        <div className="mt-2 space-y-1">
          {equip.length === 0 ? (
            <div className="text-sm text-zinc-400">None</div>
          ) : (
            equip.map((e) => {
              const name = equipNameMap[String(e.id)] ?? String(e.id);
              const { tier, isST } = deriveTierAndST(e.attr);
              return (
                <div
                  key={`${e.slot}:${e.id}`}
                  className="flex items-center justify-between text-sm"
                >
                  <div className="text-zinc-200">
                    <span className="text-zinc-200">{name}</span>
                  </div>
                  <div className="flex items-center gap-1.5">
                    {typeof tier === "number" && tier > 0 && (
                      <Badge color="gray">{roman(tier)}</Badge>
                    )}
                    {isST && <Badge color="amber">ST</Badge>}
                  </div>
                </div>
              );
            })
          )}
        </div>
      </div>
      <div>
        <SectionHeading title="Inscriptions" />
        <div className="mt-2 flex flex-wrap gap-1.5">
          {inscriptionIds.length === 0 ? (
            <div className="text-sm text-zinc-400">None</div>
          ) : (
            inscriptionIds.map((id) => {
              const label = inscriptionNameMap[String(id)] ?? String(id);
              const rarity = getInscriptionRarity(id);
              const color = rarity === "special" ? "amber" : rarity === "rare" ? "blue" : "gray";
              return (
                <Badge key={id} color={color} size="md">
                  {label}
                </Badge>
              );
            })
          )}
        </div>
      </div>
      <div>
        <SectionHeading title="Armament Buffs" />
        <div className="mt-2 space-y-1">
          {armamentIds.length === 0 ? (
            <div className="text-sm text-zinc-400">None</div>
          ) : (
            armamentIds.map((id) => {
              const name = armamentNameMap[String(id)] ?? String(id);
              const total = agg[id] ?? 0;
              const pct = `${(total * 100).toFixed(3)}%`;
              return (
                <div key={id} className="flex items-center justify-between text-sm">
                  <div className="text-zinc-200">{name}</div>
                  <div className="text-zinc-100 tabular-nums">{pct}</div>
                </div>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
}

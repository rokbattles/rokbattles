import type { Locale } from "next-intl";
import { getTranslations } from "next-intl/server";
import { resolveNames } from "@/actions/datasets";
import { EquipmentGrid } from "@/components/battle/EquipmentGrid";
import type { EquipmentToken } from "@/components/battle/EquipmentSlot";
import { InscriptionBadge } from "@/components/battle/InscriptionBadge";
import { Subheading } from "@/components/ui/heading";
import { routing } from "@/i18n/routing";
import type { ParticipantInfo } from "@/lib/types/reports";

function parseEquipment(raw?: string): EquipmentToken[] {
  if (!raw) return [];
  const inner = raw.trim().replace(/^[{\s]*|[}\s]*$/g, "");
  if (!inner) return [];
  const parts = inner.split(",");
  const out: EquipmentToken[] = [];
  for (const part of parts) {
    const segs = part.split(":");
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

function getInscriptionRarity(id: number): "common" | "rare" | "special" {
  if (id >= 1000) {
    const last = id % 10;
    if (last === 1) return "special";
    if (last === 2) return "rare";
  }
  return "common";
}

export async function BattleParticipant({
  participant,
  locale = routing.defaultLocale,
}: {
  participant?: ParticipantInfo;
  locale?: Locale;
}) {
  if (!participant) return null;

  const t = await getTranslations("battle.participant");

  const equip = parseEquipment(participant.equipment);
  const equipBySlot: Record<number, EquipmentToken | undefined> = {};

  const inscriptionIds = parseSemicolonIds(participant.inscriptions);
  const inscriptionNameMap =
    inscriptionIds.length > 0
      ? await resolveNames("inscriptions", inscriptionIds.map(String), locale)
      : {};
  for (const token of equip) equipBySlot[token.slot] = token;

  const armPairs = parseArmamentPairs(participant.armament_buffs);
  const agg: Record<number, number> = {};
  for (const { id, value } of armPairs) {
    agg[id] = (agg[id] ?? 0) + (Number.isFinite(value) ? value : 0);
  }
  const armamentIds = Object.keys(agg).map((key) => Number(key));
  const armamentNameMap =
    armamentIds.length > 0 ? await resolveNames("armaments", armamentIds.map(String), locale) : {};

  return (
    <div className="space-y-4">
      <div>
        <Subheading>{t("equipmentHeading")}</Subheading>
        <div className="mt-3 flex items-start justify-center">
          <EquipmentGrid slots={equipBySlot} />
        </div>
      </div>
      <div>
        <Subheading>{t("armamentHeading")}</Subheading>
        {inscriptionIds.length > 0 && (
          <div className="mt-2 flex flex-wrap gap-1.5">
            {inscriptionIds.map((id) => {
              const label = inscriptionNameMap[String(id)] ?? String(id);
              const rarity = getInscriptionRarity(id);
              const color = rarity === "special" ? "amber" : rarity === "rare" ? "blue" : "gray";
              return (
                <InscriptionBadge key={id} color={color}>
                  {label}
                </InscriptionBadge>
              );
            })}
          </div>
        )}
        {armamentIds.length > 0 && (
          <div className="mt-2 space-y-1">
            {armamentIds.map((id) => {
              const name = armamentNameMap[String(id)] ?? String(id);
              const total = agg[id] ?? 0;
              const pct = `${(total * 100).toFixed(3)}%`;
              return (
                <div key={id} className="flex items-center justify-between text-sm">
                  <div className="text-zinc-200">{name}</div>
                  <div className="tabular-nums text-zinc-100">{pct}</div>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
}

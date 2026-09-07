import type { CombatLabSeason } from "@/lib/combat-lab/season";
import { commanderMap, getCommanderName } from "@/lib/commander";

export type CombatLabCommanderOption = {
  id: number;
  name: string;
  talents: string[];
};

export function getCombatLabCommanderOptions(
  locale?: string,
  season: CombatLabSeason = "soc"
): CombatLabCommanderOption[] {
  const options: CombatLabCommanderOption[] = [];

  for (const [id, commander] of Object.entries(commanderMap)) {
    if (!isCombatLabCommanderId(Number(id), season)) {
      continue;
    }

    const commanderId = Number(id);
    const localizedName = getCommanderName(commanderId, locale) ?? id;
    options.push({
      id: commanderId,
      name: commander.prime ? `${localizedName} (Prime)` : localizedName,
      talents: commander.talents ?? [],
    });
  }

  return options.sort((left, right) => left.name.localeCompare(right.name));
}

export function isCombatLabCommanderId(id: number, season: CombatLabSeason = "soc") {
  const commander = commanderMap[String(id) as keyof typeof commanderMap];
  return (
    (commander?.rarity === "legendary" || commander?.rarity === "epic") &&
    (season === "soc" || commander.kvk_limit < 3)
  );
}

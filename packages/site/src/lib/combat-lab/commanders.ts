import { commanderMap, getCommanderName } from "@/lib/commander";

export type CombatLabCommanderOption = {
  id: number;
  name: string;
};

export function getLegendaryCommanderOptions(locale?: string): CombatLabCommanderOption[] {
  const options: CombatLabCommanderOption[] = [];

  for (const [id, commander] of Object.entries(commanderMap)) {
    if (commander.rarity !== "legendary") {
      continue;
    }

    const commanderId = Number(id);
    const localizedName = getCommanderName(commanderId, locale) ?? id;
    options.push({
      id: commanderId,
      name: commander.prime ? `${localizedName} (Prime)` : localizedName,
    });
  }

  return options.sort((left, right) => left.name.localeCompare(right.name));
}

export function isLegendaryCommanderId(id: number) {
  return commanderMap[String(id) as keyof typeof commanderMap]?.rarity === "legendary";
}

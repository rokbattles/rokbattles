import { commanderMap, getCommanderName, getCommanderSprites } from "@/lib/commander";

// Shares store the same English display names used by the commander selector.
const commanderIds = new Map(
  Object.entries(commanderMap).map(([id, commander]) => {
    const name = getCommanderName(Number(id), "en") ?? id;
    return [commander.prime ? `${name} (Prime)` : name, Number(id)];
  })
);

export function routeCommanderSprite(name: string): string | undefined {
  // The generated profile is [mask, rarity background, portrait]. Keep only the
  // transparent portrait, so it can sit directly on the map.
  return getCommanderSprites(commanderIds.get(name))?.find(
    (url) => !url.includes("/img_icon_HeroProfile_BG")
  );
}

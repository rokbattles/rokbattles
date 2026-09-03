const GAME_SPRITE_BASE_URL = "https://cdn.rokbattles.com/game/sprites";
const GAME_MAP_BASE_URL = "https://cdn.rokbattles.com/game/maps";
const TERRITORY_DATA_BASE_URL = "https://cdn.rokbattles.com/game/territory";

export function getGameSpriteUrl(sprite: string): string {
  return `${GAME_SPRITE_BASE_URL}/${sprite}`;
}

export function getGameSpriteUrls(sprites: string | string[] | undefined): string[] {
  if (!sprites) {
    return [];
  }

  const spriteList = Array.isArray(sprites) ? sprites : [sprites];

  return spriteList
    .filter((sprite): sprite is string => typeof sprite === "string" && sprite.length > 0)
    .map(getGameSpriteUrl);
}

export function getGameMapUrl(image: string): string {
  return `${GAME_MAP_BASE_URL}/${image}`;
}

export function getTerritoryDataBaseUrl(map: string): string {
  return `${TERRITORY_DATA_BASE_URL}/${map}/`;
}

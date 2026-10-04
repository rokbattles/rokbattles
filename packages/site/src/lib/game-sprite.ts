const GAME_SPRITE_BASE_URL = "https://cdn.rokbattles.com/game/sprites";

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

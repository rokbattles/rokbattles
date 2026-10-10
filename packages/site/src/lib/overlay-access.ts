const OVERLAY_DISCORD_IDS = ["187342661060001792", "188044055698079756", "1126322387240095824"];

export function hasOverlayAccess(discordId: string | undefined): boolean {
  return discordId !== undefined && OVERLAY_DISCORD_IDS.includes(discordId);
}

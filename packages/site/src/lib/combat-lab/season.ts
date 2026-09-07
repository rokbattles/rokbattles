export type CombatLabSeason = "soc" | "presoc";

export function combatLabSeason(pathname: string): CombatLabSeason {
  return pathname === "/combat-lab/presoc" || pathname.startsWith("/combat-lab/presoc/")
    ? "presoc"
    : "soc";
}

export function combatLabPath(season: CombatLabSeason) {
  return season === "presoc" ? "/combat-lab/presoc" : "/combat-lab";
}

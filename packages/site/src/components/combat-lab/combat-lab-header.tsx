"use client";

import { cn } from "cnfast";
import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { useExtracted } from "next-intl";
import { Listbox, ListboxOption } from "@/components/ui/listbox";
import { Text } from "@/components/ui/text";
import { type CombatLabSeason, combatLabPath, combatLabSeason } from "@/lib/combat-lab/season";

type CombatLabHeaderProps = {
  active: "explore" | "rankings";
  children?: React.ReactNode;
};

const sections = [
  { key: "explore", suffix: "" },
  { key: "rankings", suffix: "/rankings" },
] as const;

export function CombatLabHeader({ active, children }: CombatLabHeaderProps) {
  const t = useExtracted();
  const season = combatLabSeason(usePathname());
  const router = useRouter();
  const labels = { soc: t("Season of Conquest"), presoc: t("Pre-Season of Conquest") };
  const changeSeason = (next: CombatLabSeason) => {
    const path = `${combatLabPath(next)}${active === "rankings" ? "/rankings" : ""}`;
    router.push(path);
  };

  return (
    <header className="relative -mx-6 -mt-6 overflow-hidden border-zinc-950/10 border-b bg-zinc-950 text-white lg:-mx-10 lg:-mt-10 lg:rounded-t-lg dark:border-white/10">
      <div className="absolute inset-0 bg-[radial-gradient(circle_at_80%_0%,rgba(37,99,235,.28),transparent_42%),radial-gradient(circle_at_20%_100%,rgba(124,58,237,.18),transparent_38%)]" />
      <div className="relative mx-auto max-w-7xl px-4 pt-7 pb-4 sm:px-6 sm:pt-10 sm:pb-6 lg:px-8">
        <div className="max-w-xl">
          <h1 className="font-semibold text-3xl tracking-tight sm:text-4xl">{t("Combat Lab")}</h1>
          <Text className="mt-2 !text-sm/6 !text-zinc-400 sm:!text-base/6">
            {t(
              "Explore commander pairing performance with DRASTC scoring when available. Data refreshes every 8 hours."
            )}
          </Text>
        </div>
        <nav
          aria-label={t("Combat Lab sections")}
          className="mt-5 flex flex-wrap items-center gap-2 border-white/10 border-b pb-3"
        >
          <div className="dark w-full sm:w-64">
            <Listbox<CombatLabSeason>
              aria-label={t("Combat Lab season")}
              value={season}
              onChange={changeSeason}
              renderValue={(value) => (value === "presoc" ? labels.presoc : labels.soc)}
            >
              <ListboxOption value="soc">{labels.soc}</ListboxOption>
              <ListboxOption value="presoc">{labels.presoc}</ListboxOption>
            </Listbox>
          </div>
          {sections.map((section) => {
            const current = active === section.key;
            const label = section.key === "explore" ? t("Explore") : t("Rankings");
            return (
              <Link
                aria-current={current ? "page" : undefined}
                className={cn(
                  "rounded-md px-3 py-2 font-semibold text-sm/6 transition focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-400",
                  current
                    ? "bg-white text-zinc-950 shadow-sm"
                    : "text-zinc-300 hover:bg-white/10 hover:text-white"
                )}
                href={`${combatLabPath(season)}${section.suffix}`}
                key={section.key}
              >
                {label}
              </Link>
            );
          })}
        </nav>
        {children}
      </div>
    </header>
  );
}

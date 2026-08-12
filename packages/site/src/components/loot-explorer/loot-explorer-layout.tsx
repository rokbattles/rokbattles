import { cn } from "cnfast";
import Link from "next/link";
import { useExtracted } from "next-intl";
import { Heading } from "@/components/ui/heading";
import { GameTranslate } from "@/components/v1/game-translate";

export type LootExplorerSection =
  | "barbarians"
  | "barbarian-forts"
  | "baulurs"
  | "karuak-ceremony"
  | "kahars-treasure";

const sections: Array<{ key: LootExplorerSection; href: string }> = [
  { key: "barbarians", href: "/loot-explorer/barbarians" },
  { key: "barbarian-forts", href: "/loot-explorer/barbarian-forts" },
  { key: "baulurs", href: "/loot-explorer/baulurs" },
  { key: "karuak-ceremony", href: "/loot-explorer/karuak-ceremony" },
  { key: "kahars-treasure", href: "/loot-explorer/kahars-treasure" },
];

export function LootExplorerLayout({
  active,
  children,
}: {
  active: LootExplorerSection;
  children: React.ReactNode;
}) {
  const t = useExtracted();
  const sectionLabels: Record<LootExplorerSection, React.ReactNode> = {
    barbarians: <GameTranslate value="LC_COMMON_SEARCH_PVE_BAR" />,
    "barbarian-forts": <GameTranslate value="LC_COMMON_SEARCH_PVE_BAR_FORT" />,
    baulurs: t("Baulurs"),
    "karuak-ceremony": <GameTranslate value="LC_EVENT_GVE_TITLE" />,
    "kahars-treasure": <GameTranslate value="LC_KINGDOMWAR_BARBARIAN_TITLE" />,
  };

  return (
    <div className="space-y-8">
      <div className="space-y-5">
        <div className="space-y-2">
          <Heading>{t("Loot Explorer")}</Heading>
          <p className="max-w-xl text-sm/6 text-zinc-600 dark:text-zinc-400">
            {t(
              "Explore loot data from every report ROK Battles collects and processes. Data refreshes every 8 hours."
            )}
          </p>
        </div>
        <nav className="flex flex-wrap gap-2 border-zinc-950/10 border-b pb-3 dark:border-white/10">
          {sections.map((section) => (
            <Link
              key={section.key}
              href={section.href}
              className={cn(
                "rounded-md px-3 py-2 text-sm/6 font-medium",
                active === section.key
                  ? "bg-zinc-900 text-white dark:bg-white dark:text-zinc-950"
                  : "text-zinc-600 hover:bg-zinc-950/5 hover:text-zinc-950 dark:text-zinc-300 dark:hover:bg-white/10 dark:hover:text-white"
              )}
            >
              {sectionLabels[section.key]}
            </Link>
          ))}
        </nav>
      </div>
      {children}
    </div>
  );
}

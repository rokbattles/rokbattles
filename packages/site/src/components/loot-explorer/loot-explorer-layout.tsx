import { cn } from "cn";
import Link from "next/link";

import { Heading } from "@/components/ui/heading";

export type LootExplorerSection =
  | "barbarians"
  | "barbarian-forts"
  | "baulurs"
  | "karuak-ceremony"
  | "kahars-treasure"
  | "lohars-trial";

const sections: Array<{ key: LootExplorerSection; href: string }> = [
  { key: "barbarians", href: "/loot-explorer/barbarians" },
  { key: "barbarian-forts", href: "/loot-explorer/barbarian-forts" },
  { key: "baulurs", href: "/loot-explorer/baulurs" },
  { key: "lohars-trial", href: "/loot-explorer/lohars-trial" },
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
  const sectionLabels: Record<LootExplorerSection, string> = {
    barbarians: "Barbarians",
    "barbarian-forts": "Barbarian Forts",
    baulurs: "Baulurs",
    "karuak-ceremony": "Karuak Ceremony",
    "kahars-treasure": "Kahar's Treasure",
    "lohars-trial": "Lohar's Trial",
  };

  return (
    <div className="space-y-8">
      <div className="space-y-5">
        <div className="space-y-2">
          <Heading>Loot Explorer</Heading>
          <p className="max-w-xl text-sm/6 text-zinc-400">
            Explore loot data from every report ROK Battles collects and processes. Data refreshes
            every 8 hours.
          </p>
        </div>
        <nav className="flex flex-wrap gap-2 border-b pb-3 border-white/10">
          {sections.map((section) => (
            <Link
              key={section.key}
              href={section.href}
              className={cn(
                "rounded-md px-3 py-2 text-sm/6 font-medium",
                active === section.key
                  ? "bg-white text-zinc-950"
                  : "text-zinc-300 hover:bg-white/10 hover:text-white"
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

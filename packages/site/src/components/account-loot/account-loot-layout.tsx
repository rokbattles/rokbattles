"use client";

import { cn } from "cn";
import Link from "next/link";
import { useSearchParams } from "next/navigation";

import { Heading } from "@/components/ui/heading";

export type AccountLootSection =
  | "barbarians"
  | "barbarian-forts"
  | "baulurs"
  | "lohars-trial"
  | "karuak-ceremony"
  | "kahars-treasure";

const sections: Array<{ key: AccountLootSection; href: string }> = [
  { key: "barbarians", href: "/account/loot/barbarians" },
  { key: "barbarian-forts", href: "/account/loot/barbarian-forts" },
  { key: "baulurs", href: "/account/loot/baulurs" },
  { key: "lohars-trial", href: "/account/loot/lohars-trial" },
  { key: "karuak-ceremony", href: "/account/loot/karuak-ceremony" },
  { key: "kahars-treasure", href: "/account/loot/kahars-treasure" },
];

export function AccountLootLayout({
  active,
  children,
}: {
  active: AccountLootSection;
  children: React.ReactNode;
}) {
  const searchParams = useSearchParams();
  const dateParams = new URLSearchParams();
  for (const key of ["start", "end"]) {
    const value = searchParams.get(key);
    if (value) {
      dateParams.set(key, value);
    }
  }
  const dateQuery = dateParams.toString();
  const sectionLabels: Record<AccountLootSection, string> = {
    barbarians: "Barbarians",
    "barbarian-forts": "Barbarian Forts",
    baulurs: "Baulurs",
    "karuak-ceremony": "Karuak Ceremony",
    "lohars-trial": "Lohar's Trial",
    "kahars-treasure": "Kahar's Treasure",
  };

  return (
    <div className="space-y-8">
      <div className="space-y-5">
        <div className="space-y-2">
          <Heading>My Loot</Heading>
          <p className="max-w-xl text-sm/6 text-zinc-400">
            Explore loot that you have received from Barbarians, Barbarian Forts, Baulurs, Karuak
            Ceremony, Lohar's Trial, and Kahar's Treasure.
          </p>
        </div>
        <nav className="flex flex-wrap gap-2 border-b pb-3 border-white/10">
          {sections.map((section) => (
            <Link
              key={section.key}
              href={dateQuery ? `${section.href}?${dateQuery}` : section.href}
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

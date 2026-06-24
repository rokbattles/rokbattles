"use client";

import Link from "next/link";
import { useExtracted } from "next-intl";
import { Heading } from "@/components/ui/heading";
import { cn } from "@/lib/cn";

export type AccountLootSection = "barbarians" | "barbarian-forts" | "baulurs";

const sections: Array<{ key: AccountLootSection; href: string }> = [
  { key: "barbarians", href: "/account/loot/barbarians" },
  { key: "barbarian-forts", href: "/account/loot/barbarian-forts" },
  { key: "baulurs", href: "/account/loot/baulurs" },
];

export function AccountLootLayout({
  active,
  children,
}: {
  active: AccountLootSection;
  children: React.ReactNode;
}) {
  const t = useExtracted();
  const sectionLabels: Record<AccountLootSection, string> = {
    barbarians: t("Barbarians"),
    "barbarian-forts": t("Barbarian Forts"),
    baulurs: t("Baulurs"),
  };

  return (
    <div className="space-y-8">
      <div className="space-y-5">
        <div className="space-y-2">
          <Heading>{t("My Loot")}</Heading>
          <p className="max-w-xl text-sm/6 text-zinc-600 dark:text-zinc-400">
            {t(
              "Explore loot that you have received from Barbarians, Barbarian Forts, and Baulurs."
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

import { cn } from "cnfast";
import Link from "next/link";
import { Heading } from "@/components/ui/heading";

export type CombatLabSection = "explore" | "rankings";

const sections: Array<{ key: CombatLabSection; href: string; label: string }> = [
  { key: "explore", href: "/combat-lab", label: "Explore" },
  { key: "rankings", href: "/combat-lab/rankings", label: "Rankings" },
];

export function CombatLabLayout({
  active,
  children,
}: {
  active: CombatLabSection;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-8">
      <div className="space-y-5">
        <div className="max-w-xl space-y-2">
          <Heading>Combat Lab</Heading>
          <p className="text-sm/6 text-zinc-600 dark:text-zinc-400">
            Explore commander pairing performance with DRASTC scoring when available. Data refreshes
            every 8 hours.
          </p>
        </div>
        <nav
          aria-label="Combat Lab sections"
          className="flex flex-wrap gap-2 border-zinc-950/10 border-b pb-3 dark:border-white/10"
        >
          {sections.map((section) => (
            <Link
              key={section.key}
              aria-current={active === section.key ? "page" : undefined}
              href={section.href}
              className={cn(
                "rounded-md px-3 py-2 text-sm/6 font-medium",
                active === section.key
                  ? "bg-zinc-900 text-white dark:bg-white dark:text-zinc-950"
                  : "text-zinc-600 hover:bg-zinc-950/5 hover:text-zinc-950 dark:text-zinc-300 dark:hover:bg-white/10 dark:hover:text-white"
              )}
            >
              {section.label}
            </Link>
          ))}
        </nav>
      </div>
      {children}
    </div>
  );
}

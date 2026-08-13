"use client";

import { useExtracted } from "next-intl";
import { Text } from "@/components/ui/text";

export function CombatLabEmptyState({ className = "" }: { className?: string }) {
  const t = useExtracted();

  return (
    <div
      className={`flex min-h-48 items-center justify-center rounded-md border border-zinc-950/10 border-dashed bg-zinc-950/[.02] px-6 py-10 text-center dark:border-white/10 dark:bg-white/[.02] ${className}`}
    >
      <Text className="font-medium !text-sm/5 !text-zinc-700 dark:!text-zinc-200">
        {t("No data available")}
      </Text>
    </div>
  );
}

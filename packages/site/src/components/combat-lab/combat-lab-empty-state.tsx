"use client";

import { Text } from "@/components/ui/text";

export function CombatLabEmptyState({ className = "" }: { className?: string }) {
  return (
    <div
      className={`flex min-h-48 items-center justify-center rounded-md border border-dashed px-6 py-10 text-center border-white/10 bg-white/[.02] ${className}`}
    >
      <Text className="font-medium !text-sm/5 !text-zinc-200">No data available</Text>
    </div>
  );
}

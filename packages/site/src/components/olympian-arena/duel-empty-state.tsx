"use client";

import { Text } from "@/components/ui/text";

export function DuelEmptyState() {
  return (
    <Text role="status" aria-live="polite">
      No reports were found for this duel.
    </Text>
  );
}

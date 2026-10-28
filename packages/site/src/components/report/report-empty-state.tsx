"use client";

import { Text } from "@/components/ui/text";

export function ReportEmptyState() {
  return (
    <Text role="status" aria-live="polite">
      No battles were found for this hash.
    </Text>
  );
}

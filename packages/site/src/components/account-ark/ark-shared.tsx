"use client";

import type { ReactElement, ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import type { ArkLeague } from "@/lib/types/ark";

const decimalFormatter = new Intl.NumberFormat("en-US", { maximumFractionDigits: 1 });

export function useArkLeagueLabels(): Record<ArkLeague | "all", string> {
  return {
    all: "All matches",
    golden: "Golden Battleground",
    silver: "Silver Battleground",
    osiris: "Osiris League",
    practice: "Practice",
    custom: "Custom",
    unknown: "Unclassified",
  };
}

type ArkSectionHeadingProps = {
  title: string;
  description: string;
  children?: ReactNode;
};

export function ArkSectionHeading({
  title,
  description,
  children,
}: ArkSectionHeadingProps): ReactElement {
  return (
    <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
      <div>
        <Subheading>{title}</Subheading>
        <Text className="mt-1">{description}</Text>
      </div>
      {children}
    </div>
  );
}

type ArkMetricCardsProps = {
  items: { label: string; value: string; detail?: string }[];
};

export function ArkMetricCards({ items }: ArkMetricCardsProps): ReactElement {
  return (
    <dl className="grid grid-cols-2 gap-3 lg:grid-cols-4">
      {items.map((item) => (
        <div key={item.label} className="rounded-md border p-4 border-zinc-800">
          <dt className="text-sm text-zinc-400">{item.label}</dt>
          <dd className="mt-2 text-2xl font-semibold tabular-nums text-white">{item.value}</dd>
          {item.detail ? <dd className="mt-1 text-xs text-zinc-400">{item.detail}</dd> : null}
        </div>
      ))}
    </dl>
  );
}

type ArkPaginationProps = {
  page: number;
  pages: number;
  onChange: (page: number) => void;
};

export function ArkPagination({ page, pages, onChange }: ArkPaginationProps): ReactElement {
  return (
    <div className="flex items-center justify-between gap-3">
      <Text>{`Page ${String(page)} / ${String(pages)}`}</Text>
      <div className="flex gap-2">
        <Button plain disabled={page <= 1} onClick={() => onChange(page - 1)}>
          Previous
        </Button>
        <Button plain disabled={page >= pages} onClick={() => onChange(page + 1)}>
          Next
        </Button>
      </div>
    </div>
  );
}

type ArkRequestStateProps = {
  error?: boolean;
  retry?: () => void;
  children: ReactNode;
};

export function ArkRequestState({
  error = false,
  retry,
  children,
}: ArkRequestStateProps): ReactElement {
  return (
    <div
      className="mt-6 space-y-3 rounded-md border px-6 py-16 text-center border-zinc-800"
      role={error ? "alert" : "status"}
    >
      <Text>{children}</Text>
      {retry ? (
        <Button outline onClick={retry}>
          Try again
        </Button>
      ) : null}
    </div>
  );
}

export function useArkNumber(): (value: number | null | undefined) => string {
  return (value) => (value == null ? "—" : decimalFormatter.format(value));
}

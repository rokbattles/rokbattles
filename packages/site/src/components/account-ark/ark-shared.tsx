"use client";

import { useExtracted, useFormatter } from "next-intl";
import type { ReactElement, ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import type { ArkLeague } from "@/lib/types/ark";

export function useArkLeagueLabels(): Record<ArkLeague | "all", string> {
  const t = useExtracted();

  return {
    all: t("All matches"),
    golden: t("Golden Battleground"),
    silver: t("Silver Battleground"),
    osiris: t("Osiris League"),
    practice: t("Practice"),
    custom: t("Custom"),
    unknown: t("Unclassified"),
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
  const t = useExtracted();

  return (
    <div className="flex items-center justify-between gap-3">
      <Text>{t("Page {page} / {pages}", { page: String(page), pages: String(pages) })}</Text>
      <div className="flex gap-2">
        <Button plain disabled={page <= 1} onClick={() => onChange(page - 1)}>
          {t("Previous")}
        </Button>
        <Button plain disabled={page >= pages} onClick={() => onChange(page + 1)}>
          {t("Next")}
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
  const t = useExtracted();

  return (
    <div
      className="mt-6 space-y-3 rounded-md border px-6 py-16 text-center border-zinc-800"
      role={error ? "alert" : "status"}
    >
      <Text>{children}</Text>
      {retry ? (
        <Button outline onClick={retry}>
          {t("Try again")}
        </Button>
      ) : null}
    </div>
  );
}

export function useArkNumber(): (value: number | null | undefined) => string {
  const intl = useFormatter();

  return (value) => (value == null ? "—" : intl.number(value, { maximumFractionDigits: 1 }));
}

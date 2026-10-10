"use client";

import { ArrowDown, ArrowUp, ArrowUpDown } from "lucide-react";
import { useExtracted } from "next-intl";
import { type ReactElement, useState } from "react";
import {
  ArkPagination,
  ArkSectionHeading,
  useArkNumber,
} from "@/components/account-ark/ark-shared";
import { Input } from "@/components/ui/input";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Text } from "@/components/ui/text";
import type { ArkParticipant } from "@/lib/types/ark";

type ArkTeamScoresProps = { participants: ArkParticipant[] };
type ScoreKey = "score" | "occupationScore" | "provisionsScore" | "killScore" | "arkScore";

export function ArkTeamScores({ participants }: ArkTeamScoresProps): ReactElement {
  const t = useExtracted();
  const number = useArkNumber();

  const [search, setSearch] = useState("");
  const [page, setPage] = useState(1);
  const [sort, setSort] = useState<{ key: ScoreKey; descending: boolean }>({
    key: "score",
    descending: true,
  });

  const columns: { key: ScoreKey; label: string }[] = [
    { key: "score", label: t("Individual score") },
    { key: "occupationScore", label: t("Occupation") },
    { key: "provisionsScore", label: t("Provisions") },
    { key: "killScore", label: t("Kill score") },
    { key: "arkScore", label: t("Ark score") },
  ];

  const normalizedSearch = search.trim().toLocaleLowerCase();
  const rows = participants
    .filter((player) => (player.name ?? "").toLocaleLowerCase().includes(normalizedSearch))
    .toSorted((a, b) => {
      const left = a[sort.key];
      const right = b[sort.key];

      if (left == null) {
        return right == null ? a.rank - b.rank : 1;
      }

      if (right == null) {
        return -1;
      }

      return (sort.descending ? right - left : left - right) || a.rank - b.rank;
    });

  const maxScore = Math.max(1, ...participants.map((player) => player.score ?? 0));
  const pages = Math.max(1, Math.ceil(rows.length / 10));
  const currentPage = Math.min(page, pages);

  return (
    <section className="space-y-4">
      <ArkSectionHeading
        title={t("Team scores")}
        description={t("Individual scores and activity values. Select a column to sort.")}
      >
        <Input
          type="search"
          aria-label={t("Search teammates")}
          placeholder={t("Search teammates…")}
          className="sm:max-w-64"
          value={search}
          onChange={(event) => {
            setSearch(event.target.value);
            setPage(1);
          }}
        />
      </ArkSectionHeading>
      {!participants.length ? (
        <Text>{t("No teammate scores were included in this result mail.")}</Text>
      ) : (
        <>
          <Table
            dense
            className="[--gutter:--spacing(6)] lg:[--gutter:--spacing(10)] lg:[&_table]:w-full lg:[&_table]:table-fixed lg:[&_table]:min-w-240"
          >
            <TableHead>
              <TableRow>
                <TableHeader className="w-16">{t("Rank")}</TableHeader>
                <TableHeader>{t("Governor")}</TableHeader>
                {columns.map((column) => {
                  const active = column.key === sort.key;
                  const direction = sort.descending ? "descending" : "ascending";
                  const SortIcon = sort.descending ? ArrowDown : ArrowUp;
                  const Icon = active ? SortIcon : ArrowUpDown;

                  return (
                    <TableHeader
                      key={column.key}
                      className="w-36 text-right"
                      aria-sort={active ? direction : "none"}
                    >
                      <button
                        type="button"
                        className="inline-flex items-center gap-1 rounded focus-visible:outline-2 focus-visible:outline-blue-500"
                        onClick={() => {
                          setSort({ key: column.key, descending: !active || !sort.descending });
                          setPage(1);
                        }}
                      >
                        {column.label}
                        <Icon aria-hidden="true" className="size-3" />
                      </button>
                    </TableHeader>
                  );
                })}
              </TableRow>
            </TableHead>
            <TableBody>
              {rows.slice((currentPage - 1) * 10, currentPage * 10).map((player) => (
                <TableRow key={player.rank}>
                  <TableCell className="tabular-nums text-zinc-400">
                    {player.score == null ? "—" : player.rank}
                  </TableCell>
                  <TableCell>
                    <div className="truncate font-medium" title={player.name ?? undefined}>
                      {player.name ?? t("Unknown governor")}
                    </div>
                    {player.participated === false ? (
                      <div className="text-xs text-zinc-400">{t("Did not participate")}</div>
                    ) : null}
                  </TableCell>
                  {columns.map((column) => (
                    <TableCell key={column.key} className="text-right tabular-nums">
                      {number(player[column.key])}
                      {column.key === "score" && player.score != null ? (
                        <div aria-hidden="true" className="mt-1.5 h-1 rounded-full bg-zinc-800">
                          <div
                            className="h-1 rounded-full bg-blue-500"
                            style={{ width: `${(player.score / maxScore) * 100}%` }}
                          />
                        </div>
                      ) : null}
                    </TableCell>
                  ))}
                </TableRow>
              ))}
              {!rows.length ? (
                <TableRow>
                  <TableCell colSpan={7}>{t("No teammates match your search.")}</TableCell>
                </TableRow>
              ) : null}
            </TableBody>
          </Table>
          <ArkPagination page={currentPage} pages={pages} onChange={setPage} />
        </>
      )}
    </section>
  );
}

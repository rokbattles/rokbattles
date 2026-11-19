import { ArrowDownLeft, ArrowUpRight } from "lucide-react";
import { LootIcon } from "@/components/account-loot/loot-icon";
import { Badge } from "@/components/ui/badge";
import { Subheading } from "@/components/ui/heading";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Text } from "@/components/ui/text";
import { getLootSprites } from "@/lib/loot-catalog";
import type {
  CarriageActivity,
  CarriageCounterparty,
  CarriageResource,
} from "@/lib/types/carriage";

const exactAmountFormatter = new Intl.NumberFormat("en-US");
const compactAmountFormatter = new Intl.NumberFormat("en-US", {
  notation: "compact",
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

const taxFormatter = new Intl.NumberFormat("en-US", {
  style: "percent",
  maximumFractionDigits: 1,
});

const dateFormatter = new Intl.DateTimeFormat("en-US", {
  month: "2-digit",
  day: "2-digit",
  timeZone: "UTC",
});
const timeFormatter = new Intl.DateTimeFormat("en-US", {
  hour: "2-digit",
  minute: "2-digit",
  hourCycle: "h23",
  timeZone: "UTC",
});

const tradingResources = [
  { type: 1, name: "Food" },
  { type: 2, name: "Wood" },
  { type: 3, name: "Stone" },
  { type: 4, name: "Gold" },
].map((resource) => ({ ...resource, spriteUrls: getLootSprites(1, resource.type) }));

const transferSeries = [
  { key: "received", label: "Received", color: "bg-emerald-400" },
  { key: "sent", label: "Sent", color: "bg-sky-400" },
] as const;

function counterpartyName(counterparty: CarriageCounterparty) {
  return counterparty.name.trim() || `Governor ${counterparty.governorId}`;
}

function amountFor(resources: CarriageResource[], type: number) {
  return resources.find((resource) => resource.type === type)?.amount ?? 0;
}

function ResourceAmount({ amount }: { amount: number }) {
  const exact = exactAmountFormatter.format(amount);

  return (
    <span title={exact} className="relative tabular-nums">
      {amount >= 1000 ? (
        <>
          <span aria-hidden="true">{compactAmountFormatter.format(amount)}</span>
          <span className="sr-only">{exact}</span>
        </>
      ) : (
        exact
      )}
    </span>
  );
}

function ResourceAmounts({ resources }: { resources: CarriageResource[] }) {
  return (
    <dl className="@container grid grid-cols-4 gap-x-2" aria-label="Resources">
      {tradingResources.map(({ type, name, spriteUrls }) => {
        const amount = amountFor(resources, type);

        return (
          <div key={type} className="min-w-0" title={name}>
            <dt className="sr-only">{name}</dt>
            <dd className="flex flex-col items-start gap-0.5 @md:flex-row @md:items-center @md:gap-2">
              <LootIcon spriteUrls={spriteUrls} className="h-5 w-5" />
              <span
                className={`min-w-0 font-medium text-xs wrap-anywhere @md:text-sm ${amount === 0 ? "text-zinc-400" : "text-zinc-100"}`}
              >
                <ResourceAmount amount={amount} />
              </span>
            </dd>
          </div>
        );
      })}
    </dl>
  );
}

export function CarriageTotalsChart({
  received,
  sent,
}: {
  received: CarriageResource[];
  sent: CarriageResource[];
}) {
  const points = tradingResources.map((resource) => ({
    ...resource,
    received: amountFor(received, resource.type),
    sent: amountFor(sent, resource.type),
  }));

  const scale = Math.max(1, ...points.map((point) => Math.max(point.received, point.sent)));

  return (
    <section className="space-y-4" aria-labelledby="carriage-totals-title">
      <div className="flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
        <Subheading id="carriage-totals-title">Resources transferred</Subheading>
        <ul className="flex gap-4 text-xs text-zinc-300" aria-label="Transfer directions">
          {transferSeries.map(({ key, label, color }) => (
            <li key={key} className="flex items-center gap-1.5">
              <span aria-hidden="true" className={`size-2 rounded-sm ${color}`} />
              {label}
            </li>
          ))}
        </ul>
      </div>
      <dl className="divide-y divide-white/5 border-y border-white/10">
        {points.map((point) => (
          <div
            key={point.type}
            className="grid grid-cols-[5rem_minmax(0,1fr)] items-center gap-2 py-2.5 sm:grid-cols-[7rem_minmax(0,1fr)]"
          >
            <dt className="row-span-2 flex items-center gap-1.5 text-sm font-medium text-white">
              <LootIcon spriteUrls={point.spriteUrls} className="h-6 w-6" />
              {point.name}
            </dt>
            {transferSeries.map(({ key, label, color }, index) => (
              <dd
                key={key}
                className={`col-start-2 grid grid-cols-[minmax(0,1fr)_5rem] items-center gap-3 ${index === 0 ? "row-start-1" : "row-start-2"}`}
              >
                <span aria-hidden="true" className="h-2 rounded-sm bg-white/5">
                  <span
                    className={`block h-full rounded-sm ${color}`}
                    style={{ width: `${(point[key] / scale) * 100}%` }}
                  />
                </span>
                <span className="relative text-right text-xs font-medium text-zinc-200 sm:text-sm">
                  <span className="sr-only">{label}: </span>
                  <ResourceAmount amount={point[key]} />
                </span>
              </dd>
            ))}
          </div>
        ))}
      </dl>
    </section>
  );
}

function CounterpartyRanking({
  direction,
  entries,
}: {
  direction: CarriageActivity["direction"];
  entries: CarriageCounterparty[];
}) {
  const inbound = direction === "received";
  const Icon = inbound ? ArrowDownLeft : ArrowUpRight;

  return (
    <section className="min-w-0 rounded-md border border-zinc-800">
      <div className="flex items-center gap-2 border-b border-zinc-800 px-4 py-3">
        <Icon
          aria-hidden="true"
          className={inbound ? "size-4 text-emerald-400" : "size-4 text-sky-400"}
        />
        <Subheading>{inbound ? "Top senders" : "Top recipients"}</Subheading>
      </div>
      {entries.length === 0 ? (
        <Text className="px-4 py-3">
          {inbound ? "No incoming transfers yet." : "No outgoing transfers yet."}
        </Text>
      ) : (
        <ul className="divide-y divide-white/5 text-sm">
          {entries.map((entry) => (
            <li key={entry.governorId} className="space-y-1 px-4 py-2.5">
              <span className="block font-medium text-white break-words">
                {counterpartyName(entry)}
              </span>
              <ResourceAmounts resources={entry.resources} />
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

export function CarriageRankings({
  received,
  sent,
}: {
  received: CarriageCounterparty[];
  sent: CarriageCounterparty[];
}) {
  return (
    <div className="grid gap-4 md:grid-cols-2">
      <CounterpartyRanking direction="received" entries={received} />
      <CounterpartyRanking direction="sent" entries={sent} />
    </div>
  );
}

function TransferTime({ millis }: { millis: number }) {
  return (
    <time dateTime={new Date(millis).toISOString()}>
      {`${dateFormatter.format(millis)} at ${timeFormatter.format(millis)} UTC`}
    </time>
  );
}

function formatReportedTax(tax: CarriageActivity["tax"]) {
  if (!tax) {
    return;
  }

  const min = taxFormatter.format(tax.min);
  const max = taxFormatter.format(tax.max);

  return min === max ? min : `${min.replace("%", "")}-${max}`;
}

function CarriageActivityRow({ entry }: { entry: CarriageActivity }) {
  const inbound = entry.direction === "received";
  const spansMinutes = Math.floor(entry.oldestAt / 60_000) !== Math.floor(entry.newestAt / 60_000);
  const tax = formatReportedTax(entry.tax);

  return (
    <TableRow>
      <TableCell className="whitespace-normal">
        <div className="flex flex-wrap items-baseline gap-x-2 gap-y-1">
          <span className="font-medium text-white wrap-anywhere">{counterpartyName(entry)}</span>
          <Badge color={inbound ? "emerald" : "sky"}>{inbound ? "Received" : "Sent"}</Badge>
        </div>
        <div className="mt-1 text-xs text-zinc-400">
          {spansMinutes ? (
            <>
              <TransferTime millis={entry.oldestAt} />
              {" – "}
            </>
          ) : null}
          <TransferTime millis={entry.newestAt} />
          {entry.reportCount > 1 ? ` · Consolidated ${entry.reportCount} reports` : null}
          {tax ? ` · ${tax} Tax` : null}
        </div>
      </TableCell>
      {tradingResources.map(({ type }) => (
        <TableCell key={type}>
          <ResourceAmount amount={amountFor(entry.resources, type)} />
        </TableCell>
      ))}
    </TableRow>
  );
}

export function CarriageActivityTable({ activity }: { activity: CarriageActivity[] }) {
  return (
    <Table
      role="region"
      aria-label="Transfer log table"
      tabIndex={0}
      className="[&_table]:w-full [&_table]:min-w-[48rem] [&_table]:table-fixed"
    >
      <caption className="sr-only">Transfer log</caption>
      <colgroup>
        <col />
        {tradingResources.map(({ type }) => (
          <col key={type} className="w-28" />
        ))}
      </colgroup>
      <TableHead>
        <TableRow>
          <TableHeader scope="col">Transfer</TableHeader>
          {tradingResources.map(({ type, name, spriteUrls }) => (
            <TableHeader key={type} scope="col">
              <span className="flex items-center gap-2">
                <LootIcon spriteUrls={spriteUrls} className="h-7 w-7" />
                {name}
              </span>
            </TableHeader>
          ))}
        </TableRow>
      </TableHead>
      <TableBody>
        {activity.map((entry) => (
          <CarriageActivityRow key={entry.id} entry={entry} />
        ))}
      </TableBody>
    </Table>
  );
}

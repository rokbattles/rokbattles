"use client";

import { Tabs } from "@base-ui/react/tabs";
import { useState } from "react";
import { PairingsEfficiencyChart } from "@/components/my-pairings/pairings-efficiency-chart";
import { PairingsTradeRateChart } from "@/components/my-pairings/pairings-trade-rate-chart";
import { Subheading } from "@/components/ui/heading";
import { Text } from "@/components/ui/text";
import type { PairingAggregate } from "@/lib/pairings";

const charts = [
  {
    tab: "Performance",
    title: "Trade performance",
    description: "Compare pairings by their average casualties per battle and KP trade ratio.",
  },
  {
    tab: "Positive trades",
    title: "Positive trade rate",
    description:
      "See how often each pairing gains more KP than it loses. Whiskers show the estimated 95% confidence interval.",
  },
];

export function PairingsCharts({ items, query }: { items: PairingAggregate[]; query: string }) {
  const [selected, setSelected] = useState(0);

  return (
    <Tabs.Root
      render={<section />}
      value={selected}
      onValueChange={(value) => setSelected(value === 1 ? 1 : 0)}
      className="min-w-0 rounded-md border border-zinc-800 p-4 sm:p-5"
    >
      <div className="flex flex-col justify-between gap-4 lg:flex-row lg:items-start">
        <div className="space-y-1">
          <Subheading>{charts[selected].title}</Subheading>
          <Text className="max-w-3xl text-sm/6">{charts[selected].description}</Text>
        </div>
        <Tabs.List
          aria-label="Pairing charts"
          className="flex shrink-0 self-start gap-4 border-b border-zinc-800 text-xs"
        >
          {charts.map((chart, index) => (
            <Tabs.Tab
              key={chart.tab}
              value={index}
              className="-mb-px border-b-2 border-transparent pb-2 pt-1 text-zinc-400 data-active:border-white data-active:text-white focus-visible:rounded-sm focus-visible:outline-2 focus-visible:outline-blue-500"
            >
              {chart.tab}
            </Tabs.Tab>
          ))}
        </Tabs.List>
      </div>
      <Tabs.Panel value={0}>
        <PairingsEfficiencyChart items={items} query={query} />
      </Tabs.Panel>
      <Tabs.Panel value={1}>
        <PairingsTradeRateChart items={items} query={query} />
      </Tabs.Panel>
    </Tabs.Root>
  );
}

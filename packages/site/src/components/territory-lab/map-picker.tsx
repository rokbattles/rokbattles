"use client";

import Image from "next/image";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Description, Field, Label } from "@/components/ui/fieldset";
import { Heading, Subheading } from "@/components/ui/heading";
import { Input } from "@/components/ui/input";
import { Radio, RadioField, RadioGroup } from "@/components/ui/radio";
import { Text } from "@/components/ui/text";
import { CDN, type MapSummary, type Mode, SEASONS } from "@/lib/territory-lab/types";

export function MapPicker({
  maps,
  open,
}: {
  maps: MapSummary[];
  open: (map: string, mode: Mode) => void;
}) {
  const [mode, setMode] = useState<Mode>("territory");
  const [kingdom, setKingdom] = useState("");
  const kingdomId = /^\d{4}$/.test(kingdom) ? Number(kingdom) : 0;
  const home = kingdomId >= 1001 ? ((kingdomId - 1001) % 4) + 1 : null;

  return (
    <div className="space-y-8">
      <header className="space-y-2">
        <Heading>Territory Lab</Heading>
      </header>
      <RadioGroup
        aria-label="Planner mode"
        value={mode}
        onChange={(value) => setMode(value === "baulur" ? "baulur" : "territory")}
      >
        <div className="grid items-start gap-6 sm:grid-cols-2">
          <RadioField>
            <Radio value="territory" />
            <Label>Territory Planner</Label>
            <Description>Build, draw, and coordinate your alliances.</Description>
          </RadioField>
          <RadioField>
            <Radio value="baulur" />
            <Label>Baulur Planner</Label>
            <Description>Plan up to five routes.</Description>
          </RadioField>
        </div>
      </RadioGroup>
      <section
        aria-labelledby="find-map-heading"
        className="space-y-4 border-y border-zinc-950/10 py-6 dark:border-white/10"
      >
        <div>
          <Subheading id="find-map-heading">Find my map</Subheading>
          <Text>Enter your home kingdom number to find its Preparation Season map.</Text>
        </div>
        <div className="flex flex-wrap items-end gap-5">
          <Field className="w-48">
            <Label>Kingdom number</Label>
            <Input
              inputMode="numeric"
              maxLength={4}
              placeholder="2804"
              value={kingdom}
              onChange={(e) => setKingdom(e.target.value.replace(/\D/g, "").slice(0, 4))}
            />
          </Field>
          <div aria-live="polite">
            {home ? (
              <div className="flex flex-wrap items-center gap-4">
                <Text>
                  Kingdom {kingdomId} uses Preparation Season {home}.
                </Text>
                <Button outline onClick={() => open(`Sever_Map_G1_${home}_v2`, mode)}>
                  Open my map
                </Button>
              </div>
            ) : kingdom.length === 4 ? (
              <Text>Enter a kingdom from 1001 to 9999.</Text>
            ) : null}
          </div>
        </div>
      </section>
      {SEASONS.filter(([season]) => mode === "territory" || season === "preparation").map(
        ([season, title]) => (
          <section key={season} aria-label={title} className="space-y-4">
            <Subheading>{title}</Subheading>
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-4">
              {maps
                .filter((map) => map.season === season)
                .map((map) => (
                  <Button
                    outline
                    key={map.map}
                    className="min-w-0 flex-col items-stretch overflow-hidden p-0 text-left sm:p-0"
                    onClick={() => open(map.map, mode)}
                  >
                    <Image
                      src={`${CDN}/${map.map}/overview.webp`}
                      alt=""
                      width={320}
                      height={200}
                      unoptimized
                      crossOrigin="anonymous"
                      loading={season === "preparation" ? "eager" : "lazy"}
                      className="h-36 w-full shrink-0 rounded-t-md object-cover"
                    />
                    <span className="grid w-full grid-rows-[1.5rem_1rem] px-3 py-2">
                      <span className="truncate text-sm/6" title={map.name}>
                        {map.name}
                      </span>
                      <span className="text-xs/4 font-normal text-zinc-500 dark:text-zinc-400">
                        {!map.categories.includes("structures") && "Structures unavailable"}
                      </span>
                    </span>
                  </Button>
                ))}
            </div>
          </section>
        )
      )}
    </div>
  );
}

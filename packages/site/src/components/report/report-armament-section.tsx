"use client";

import Image from "next/image";
import { Fragment, useId } from "react";
import {
  DescriptionDetails,
  DescriptionList,
  DescriptionTerm,
} from "@/components/ui/description-list";
import { Subheading } from "@/components/ui/heading";
import { getArmamentInfo } from "@/hooks/use-armament-name";
import { getGameSpriteUrl } from "@/lib/game-sprite";
import type { ArmamentBuff } from "@/lib/report/parsers";

type ReportArmamentSectionProps = {
  buffs: ArmamentBuff[];
};

export function ReportArmamentSection({ buffs }: ReportArmamentSectionProps) {
  const filterId = useId();

  if (buffs.length === 0) {
    return null;
  }

  const rows = buffs.map(({ id, value }) => {
    const info = getArmamentInfo(id);

    return {
      id,
      name: info?.name ?? `Armament ${(id ?? "?").toString()}`,
      value: `${(Number(value ?? 0) * 100).toFixed(2)}%`,
      icon: info?.icon,
    };
  });

  const iconColors = new Set(rows.flatMap(({ icon }) => (icon ? [icon.color] : [])));

  return (
    <div className="flex flex-col gap-2">
      <Subheading>Armament Info</Subheading>
      <DescriptionList>
        {rows.map(({ id, name, value, icon }) => (
          <Fragment key={id}>
            <DescriptionTerm className="flex items-center gap-1.5 border-none! py-1!">
              {icon ? (
                <Image
                  alt=""
                  src={getGameSpriteUrl(icon.sprite)}
                  width={14}
                  height={14}
                  className="size-3.5 shrink-0 object-contain"
                  style={{ filter: `url(#${filterId}-${icon.color.slice(1)})` }}
                />
              ) : null}
              <span>{name}</span>
            </DescriptionTerm>
            <DescriptionDetails className="pb-1! pt-1! border-none! sm:text-right tabular-nums">
              {value}
            </DescriptionDetails>
          </Fragment>
        ))}
      </DescriptionList>
      <svg aria-hidden="true" className="absolute size-0">
        <defs>
          {Array.from(iconColors, (color) => (
            <filter key={color} id={`${filterId}-${color.slice(1)}`}>
              <feFlood floodColor={color} result="tint" />
              <feComposite in="tint" in2="SourceGraphic" operator="in" />
            </filter>
          ))}
        </defs>
      </svg>
    </div>
  );
}

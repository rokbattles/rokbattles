import Image from "next/image";
import { RESOURCE_SPRITES } from "@/lib/territory/assets";
import type { ResourceKind } from "@/lib/territory/types";

export function ResourceIcon({ kind }: { kind: ResourceKind | "credits" }) {
  return (
    <Image
      alt=""
      aria-hidden="true"
      className="size-5 shrink-0 object-contain"
      height={20}
      src={RESOURCE_SPRITES[kind]}
      width={20}
    />
  );
}

import clsx from "clsx";
import type { ComponentPropsWithoutRef } from "react";

export const bodyText = "text-base/7 sm:text-sm/7";
const tones = { muted: "text-zinc-400", accent: "text-orange-400" };

type TextProps = ComponentPropsWithoutRef<"p"> & {
  tone?: keyof typeof tones;
};

export function Text({ tone = "muted", className, ...props }: TextProps) {
  return <p {...props} className={clsx("text-pretty", bodyText, tones[tone], className)} />;
}

import { cn } from "cn";
import type { ComponentProps } from "react";
import { SiteLink } from "../site-link";

import { bodyText } from "./text";

type LinkProps = ComponentProps<typeof SiteLink>;

export function Link({ href, className, ...props }: LinkProps) {
  return (
    <SiteLink
      {...props}
      href={href}
      className={cn(
        "inline-flex min-h-11 touch-manipulation items-center gap-2 text-zinc-400 hover:text-white focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-orange-400 [&>svg]:size-4 [&>svg]:shrink-0",
        bodyText,
        className
      )}
    />
  );
}

import { cn } from "cn";
import type { ComponentProps } from "react";
import { Link as RouterLink } from "react-router";

import { bodyText } from "./text";

type LinkProps = Omit<ComponentProps<typeof RouterLink>, "to"> & { href: string };

export function Link({ href, className, ...props }: LinkProps) {
  return (
    <RouterLink
      {...props}
      to={href}
      className={cn(
        "inline-flex min-h-11 touch-manipulation items-center gap-2 text-zinc-400 hover:text-white focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-orange-400 [&>svg]:size-4 [&>svg]:shrink-0",
        bodyText,
        className
      )}
    />
  );
}

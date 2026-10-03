import NextLink from "next/link";
import type { ComponentProps } from "react";

export function SiteLink({ href, ...props }: ComponentProps<"a"> & { href: string }) {
  if (href.startsWith("/") && !href.startsWith("//") && !href.endsWith(".mobileconfig")) {
    return <NextLink href={href} {...props} />;
  }

  return <a href={href} {...props} />;
}

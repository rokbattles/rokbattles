import NextLink, { type LinkProps } from "next/link";
import type React from "react";
import { forwardRef } from "react";

export const Link = forwardRef(function Link(
  props: LinkProps & React.ComponentPropsWithoutRef<"a">,
  ref: React.ForwardedRef<HTMLAnchorElement>
) {
  return <NextLink {...props} ref={ref} />;
});

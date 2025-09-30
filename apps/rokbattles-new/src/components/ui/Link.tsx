"use client";

import * as Headless from "@headlessui/react";
import type { Route } from "next";
import type { LinkProps as NextLinkProps } from "next/link";
import type React from "react";
import { forwardRef } from "react";
import { Link as IntlLink } from "@/i18n/navigation";

type Intersect<T> = { [K in keyof T]: T[K] } & {};
type IntlBaseProps = Omit<React.ComponentProps<typeof IntlLink>, "href">;
type TypedHref<H extends Route> = NextLinkProps<H>["href"];

export type LinkProps<H extends Route> = Intersect<IntlBaseProps & { href: TypedHref<H> }>;

function LinkImpl<H extends Route>(
  { href, ...rest }: LinkProps<H>,
  ref: React.ForwardedRef<HTMLAnchorElement>
) {
  return (
    <Headless.DataInteractive>
      <IntlLink ref={ref} href={href as React.ComponentProps<typeof IntlLink>["href"]} {...rest} />
    </Headless.DataInteractive>
  );
}

export const Link = forwardRef(LinkImpl) as <H extends Route>(
  props: LinkProps<H> & { ref?: React.Ref<HTMLAnchorElement> }
) => React.ReactElement;

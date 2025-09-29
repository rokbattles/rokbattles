"use client";

import * as Headless from "@headlessui/react";
import type { Route } from "next";
import type { LinkProps as NextLinkProps } from "next/link";
import {
  type ComponentProps,
  type ForwardedRef,
  forwardRef,
  type ReactElement,
  type Ref,
} from "react";
import { Link as IntlLink } from "@/i18n/navigation";

type Intersect<T> = { [K in keyof T]: T[K] } & {};
type IntlBaseProps = Omit<ComponentProps<typeof IntlLink>, "href">;
type TypedHref<H extends Route> = NextLinkProps<H>["href"];

export type LinkProps<H extends Route> = Intersect<IntlBaseProps & { href: TypedHref<H> }>;

function LinkImpl<H extends Route>(
  { href, ...rest }: LinkProps<H>,
  ref: ForwardedRef<HTMLAnchorElement>
) {
  return (
    <Headless.DataInteractive>
      <IntlLink ref={ref} href={href as ComponentProps<typeof IntlLink>["href"]} {...rest} />
    </Headless.DataInteractive>
  );
}

export const Link = forwardRef(LinkImpl) as <H extends Route>(
  props: LinkProps<H> & { ref?: Ref<HTMLAnchorElement> }
) => ReactElement;

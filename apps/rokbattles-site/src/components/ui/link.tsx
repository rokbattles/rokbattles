"use client";

import * as Headless from "@headlessui/react";
import type React from "react";
import { forwardRef } from "react";
import { Link as IntlLink } from "@/i18n/navigation";

type IntlLinkProps = React.ComponentProps<typeof IntlLink>;
type IntlLinkRef = React.ComponentRef<typeof IntlLink>;

const IntlLinkAdapter = forwardRef<IntlLinkRef, IntlLinkProps>(
  function IntlLinkAdapter(props, ref) {
    return <IntlLink {...props} ref={ref} />;
  }
);

export const Link = forwardRef<IntlLinkRef, IntlLinkProps>(function Link(props, ref) {
  // @ts-expect-error - next-intl types are not compatible with headless ui. Runtime works fine
  return <Headless.DataInteractive as={IntlLinkAdapter} {...props} ref={ref} />;
});

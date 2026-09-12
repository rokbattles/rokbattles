import type { ComponentProps } from "react";
import { Link as RouterLink } from "react-router";

export function Link({
  href,
  ...props
}: { href: string } & Omit<ComponentProps<typeof RouterLink>, "to">) {
  return <RouterLink to={href} {...props} />;
}

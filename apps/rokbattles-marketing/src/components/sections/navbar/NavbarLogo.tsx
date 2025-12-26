import Link from "next/link";
import type { ComponentProps } from "react";
import { cn } from "@/lib/cn";

export function NavbarLogo({
  className,
  href,
  ...props
}: { href: string } & Omit<ComponentProps<"a">, "href">) {
  return <Link href={href} {...props} className={cn("inline-flex items-stretch", className)} />;
}

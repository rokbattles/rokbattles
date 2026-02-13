"use client";

import { useTranslations } from "next-intl";
import type React from "react";
import { cn } from "@/lib/cn";
import { Button } from "./button";

export function Pagination({
  "aria-label": ariaLabel,
  className,
  ...props
}: React.ComponentPropsWithoutRef<"nav">) {
  const t = useTranslations("pagination");
  const resolvedLabel = ariaLabel ?? t("navigation");
  return <nav aria-label={resolvedLabel} {...props} className={cn(className, "flex gap-x-2")} />;
}

export function PaginationPrevious({
  href = null,
  className,
  children,
}: React.PropsWithChildren<{ href?: string | null; className?: string }>) {
  const t = useTranslations("pagination");
  const label = children ?? t("previous");
  return (
    <span className={cn(className, "grow basis-0")}>
      <Button
        {...(href === null ? { disabled: true } : { href })}
        plain
        aria-label={t("previousPage")}
      >
        <svg
          className="stroke-current"
          data-slot="icon"
          viewBox="0 0 16 16"
          fill="none"
          aria-hidden="true"
        >
          <path
            d="M2.75 8H13.25M2.75 8L5.25 5.5M2.75 8L5.25 10.5"
            strokeWidth={1.5}
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
        {label}
      </Button>
    </span>
  );
}

export function PaginationNext({
  href = null,
  className,
  children,
}: React.PropsWithChildren<{ href?: string | null; className?: string }>) {
  const t = useTranslations("pagination");
  const label = children ?? t("next");
  return (
    <span className={cn(className, "flex grow basis-0 justify-end")}>
      <Button {...(href === null ? { disabled: true } : { href })} plain aria-label={t("nextPage")}>
        {label}
        <svg
          className="stroke-current"
          data-slot="icon"
          viewBox="0 0 16 16"
          fill="none"
          aria-hidden="true"
        >
          <path
            d="M13.25 8L2.75 8M13.25 8L10.75 10.5M13.25 8L10.75 5.5"
            strokeWidth={1.5}
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      </Button>
    </span>
  );
}

export function PaginationList({ className, ...props }: React.ComponentPropsWithoutRef<"span">) {
  return <span {...props} className={cn(className, "hidden items-baseline gap-x-2 sm:flex")} />;
}

export function PaginationPage({
  href,
  className,
  current = false,
  children,
}: React.PropsWithChildren<{ href: string; className?: string; current?: boolean }>) {
  const t = useTranslations("pagination");
  const pageLabel = t("pageLabel", { page: String(children) });
  return (
    <Button
      href={href}
      plain
      aria-label={pageLabel}
      aria-current={current ? "page" : undefined}
      className={cn(
        className,
        "min-w-9 before:absolute before:-inset-px before:rounded-lg",
        current && "before:bg-zinc-950/5 dark:before:bg-white/10"
      )}
    >
      <span className="-mx-0.5">{children}</span>
    </Button>
  );
}

export function PaginationGap({
  className,
  children = <>&hellip;</>,
  ...props
}: React.ComponentPropsWithoutRef<"span">) {
  return (
    <span
      aria-hidden="true"
      {...props}
      className={cn(
        className,
        "w-9 text-center text-sm/6 font-semibold text-zinc-950 select-none dark:text-white"
      )}
    >
      {children}
    </span>
  );
}

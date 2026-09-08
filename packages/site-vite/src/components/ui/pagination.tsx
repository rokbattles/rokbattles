import clsx from "clsx";
import { ArrowLeft, ArrowRight } from "lucide-react";
import type React from "react";
import { Button } from "./button";

export function Pagination({
  "aria-label": ariaLabel = "Page navigation",
  className,
  ...props
}: React.ComponentPropsWithoutRef<"nav">) {
  return <nav aria-label={ariaLabel} {...props} className={clsx(className, "flex gap-x-2")} />;
}

export function PaginationPrevious({
  href = null,
  className,
  children = "Previous",
}: React.PropsWithChildren<{ href?: string | null; className?: string }>) {
  return (
    <span className={clsx(className, "grow basis-0")}>
      <Button {...(href === null ? { disabled: true } : { href })} plain aria-label="Previous page">
        <ArrowLeft aria-hidden="true" />
        {children}
      </Button>
    </span>
  );
}

export function PaginationNext({
  href = null,
  className,
  children = "Next",
}: React.PropsWithChildren<{ href?: string | null; className?: string }>) {
  return (
    <span className={clsx(className, "flex grow basis-0 justify-end")}>
      <Button {...(href === null ? { disabled: true } : { href })} plain aria-label="Next page">
        {children}
        <ArrowRight aria-hidden="true" />
      </Button>
    </span>
  );
}

export function PaginationList({ className, ...props }: React.ComponentPropsWithoutRef<"span">) {
  return <span {...props} className={clsx(className, "hidden items-baseline gap-x-2 sm:flex")} />;
}

export function PaginationPage({
  href,
  className,
  current = false,
  children,
}: React.PropsWithChildren<{ href: string; className?: string; current?: boolean }>) {
  return (
    <Button
      href={href}
      plain
      aria-label={`Page ${children}`}
      aria-current={current ? "page" : undefined}
      className={clsx(
        className,
        "min-w-9 before:absolute before:-inset-px before:rounded-lg",
        current && "before:bg-white/10"
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
      className={clsx(className, "w-9 text-center text-sm/6 font-semibold select-none text-white")}
    >
      {children}
    </span>
  );
}

import { cn } from "cn";
import type React from "react";
import { createContext, use, useMemo, useState } from "react";
import { Link } from "./link";

const TableContext = createContext<{
  bleed: boolean;
  dense: boolean;
  grid: boolean;
  striped: boolean;
}>({
  bleed: false,
  dense: false,
  grid: false,
  striped: false,
});

export function Table({
  bleed = false,
  dense = false,
  grid = false,
  striped = false,
  className,
  children,
  ...props
}: {
  bleed?: boolean;
  dense?: boolean;
  grid?: boolean;
  striped?: boolean;
} & React.ComponentPropsWithoutRef<"div">) {
  const context = useMemo(() => ({ bleed, dense, grid, striped }), [bleed, dense, grid, striped]);
  return (
    <TableContext value={context}>
      <div className="flow-root">
        <div
          {...props}
          className={cn(className, "-mx-(--gutter) overflow-x-auto whitespace-nowrap")}
        >
          <div className={cn("inline-block min-w-full align-middle", !bleed && "sm:px-(--gutter)")}>
            <table className="min-w-full text-left text-sm/6 text-white">{children}</table>
          </div>
        </div>
      </div>
    </TableContext>
  );
}

export function TableHead({ className, ...props }: React.ComponentPropsWithoutRef<"thead">) {
  return <thead {...props} className={cn(className, "text-zinc-400")} />;
}

export function TableBody(props: React.ComponentPropsWithoutRef<"tbody">) {
  return <tbody {...props} />;
}

const TableRowContext = createContext<{ href?: string; target?: string; title?: string }>({
  href: undefined,
  target: undefined,
  title: undefined,
});

export function TableRow({
  href,
  target,
  title,
  className,
  ...props
}: { href?: string; target?: string; title?: string } & React.ComponentPropsWithoutRef<"tr">) {
  const { striped } = use(TableContext);

  const context = useMemo(() => ({ href, target, title }), [href, target, title]);
  return (
    <TableRowContext value={context}>
      <tr
        {...props}
        className={cn(
          className,
          href &&
            "has-[[data-row-link]:focus-visible]:outline-2 has-[[data-row-link]:focus-visible]:-outline-offset-2 has-[[data-row-link]:focus-visible]:outline-blue-500 focus-within:bg-white/2.5",
          striped && "even:bg-white/2.5",
          href && striped && "hover:bg-white/5",
          href && !striped && "hover:bg-white/2.5"
        )}
      />
    </TableRowContext>
  );
}

export function TableHeader({ className, ...props }: React.ComponentPropsWithoutRef<"th">) {
  const { bleed, grid } = use(TableContext);

  return (
    <th
      {...props}
      className={cn(
        className,
        "border-b px-4 py-2 font-medium first:pl-(--gutter,--spacing(2)) last:pr-(--gutter,--spacing(2)) border-b-white/10",
        grid && "border-l first:border-l-0 border-l-white/5",
        !bleed && "sm:first:pl-1 sm:last:pr-1"
      )}
    />
  );
}

export function TableCell({ className, children, ...props }: React.ComponentPropsWithoutRef<"td">) {
  const { bleed, dense, grid, striped } = use(TableContext);
  const { href, target, title } = use(TableRowContext);
  const [cellRef, setCellRef] = useState<HTMLElement | null>(null);

  return (
    <td
      ref={href ? setCellRef : undefined}
      {...props}
      className={cn(
        className,
        "relative px-4 first:pl-(--gutter,--spacing(2)) last:pr-(--gutter,--spacing(2))",
        !striped && "border-b border-white/5",
        grid && "border-l first:border-l-0 border-l-white/5",
        dense ? "py-2.5" : "py-4",
        !bleed && "sm:first:pl-1 sm:last:pr-1"
      )}
    >
      {href && (
        <Link
          data-row-link
          href={href}
          target={target}
          aria-label={title}
          tabIndex={cellRef?.previousElementSibling === null ? 0 : -1}
          className="absolute inset-0 focus:outline-hidden"
        />
      )}
      {children}
    </td>
  );
}

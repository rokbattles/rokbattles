import { Menu as BaseMenu } from "@base-ui/react/menu";
import clsx from "clsx";
import type React from "react";
import { Button } from "./button";
import { Link } from "./link";

export function Dropdown(props: BaseMenu.Root.Props) {
  return <BaseMenu.Root {...props} />;
}

export function DropdownButton({
  render = <Button />,
  ...props
}: React.ComponentProps<typeof BaseMenu.Trigger>) {
  return <BaseMenu.Trigger render={render} {...props} />;
}

type DropdownMenuProps = Omit<React.ComponentProps<typeof BaseMenu.Popup>, "className"> & {
  className?: string;
  side?: BaseMenu.Positioner.Props["side"];
  align?: BaseMenu.Positioner.Props["align"];
  sideOffset?: number;
};

export function DropdownMenu({
  side = "bottom",
  align = "start",
  sideOffset = 8,
  className,
  ...props
}: DropdownMenuProps) {
  return (
    <BaseMenu.Portal>
      <BaseMenu.Positioner side={side} align={align} sideOffset={sideOffset} collisionPadding={8}>
        <BaseMenu.Popup
          {...props}
          className={clsx(
            "isolate max-h-(--available-height) min-w-40 w-max overflow-y-auto rounded-xl bg-zinc-800/75 p-1 text-white shadow-lg ring-1 ring-white/10 ring-inset outline outline-transparent backdrop-blur-xl scheme-dark focus:outline-hidden",
            "supports-[grid-template-columns:subgrid]:grid supports-[grid-template-columns:subgrid]:grid-cols-[auto_1fr_1.5rem_0.5rem_auto]",
            "transition-opacity duration-100 data-starting-style:opacity-0 data-ending-style:opacity-0 motion-reduce:transition-none",
            className
          )}
        />
      </BaseMenu.Positioner>
    </BaseMenu.Portal>
  );
}

type DropdownItemProps = Omit<React.ComponentProps<typeof BaseMenu.Item>, "className"> & {
  className?: string;
  href?: string;
  target?: React.HTMLAttributeAnchorTarget;
  rel?: string;
};

export function DropdownItem({ className, href, target, rel, ...props }: DropdownItemProps) {
  const classes = clsx(
    className,
    "group cursor-default rounded-lg px-3.5 py-2.5 focus:outline-hidden sm:px-3 sm:py-1.5 text-left text-base/6 sm:text-sm/6 forced-colors:text-[CanvasText] data-highlighted:bg-blue-500 data-highlighted:text-white data-disabled:opacity-50 forced-color-adjust-none forced-colors:data-highlighted:bg-[Highlight] forced-colors:data-highlighted:text-[HighlightText] forced-colors:data-highlighted:[&>svg]:text-[HighlightText] col-span-full grid grid-cols-[auto_1fr_1.5rem_0.5rem_auto] items-center supports-[grid-template-columns:subgrid]:grid-cols-subgrid [&>svg]:col-start-1 [&>svg]:row-start-1 [&>svg]:mr-2.5 [&>svg]:-ml-0.5 [&>svg]:size-5 sm:[&>svg]:mr-2 sm:[&>svg]:size-4 *:data-[slot=avatar]:mr-2.5 *:data-[slot=avatar]:-ml-1 *:data-[slot=avatar]:size-6 sm:*:data-[slot=avatar]:mr-2 sm:*:data-[slot=avatar]:size-5 text-white [&>svg]:text-zinc-400 data-highlighted:[&>svg]:text-white"
  );

  return (
    <BaseMenu.Item
      render={href === undefined ? undefined : <Link href={href} target={target} rel={rel} />}
      {...props}
      className={classes}
    />
  );
}

export function DropdownHeader({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return <div {...props} className={clsx(className, "col-span-5 px-3.5 pt-2.5 pb-1 sm:px-3")} />;
}

export function DropdownSection({
  className,
  ...props
}: { className?: string } & Omit<BaseMenu.Group.Props, "className">) {
  return (
    <BaseMenu.Group
      {...props}
      className={clsx(
        className,
        "col-span-full supports-[grid-template-columns:subgrid]:grid supports-[grid-template-columns:subgrid]:grid-cols-[auto_1fr_1.5rem_0.5rem_auto]"
      )}
    />
  );
}

export function DropdownHeading({
  className,
  ...props
}: { className?: string } & Omit<BaseMenu.GroupLabel.Props, "className">) {
  return (
    <BaseMenu.GroupLabel
      {...props}
      className={clsx(
        className,
        "col-span-full grid grid-cols-[1fr_auto] gap-x-12 px-3.5 pt-2 pb-1 text-sm/5 font-medium sm:px-3 sm:text-xs/5 text-zinc-400"
      )}
    />
  );
}

export function DropdownDivider({
  className,
  ...props
}: { className?: string } & Omit<BaseMenu.Separator.Props, "className">) {
  return (
    <BaseMenu.Separator
      {...props}
      className={clsx(
        className,
        "col-span-full mx-3.5 my-1 h-px border-0 sm:mx-3 forced-colors:bg-[CanvasText] bg-white/10"
      )}
    />
  );
}

export function DropdownLabel({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return (
    <div {...props} data-slot="label" className={clsx(className, "col-start-2 row-start-1")} />
  );
}

export function DropdownDescription({
  className,
  ...props
}: { className?: string } & React.ComponentProps<"p">) {
  return (
    <p
      data-slot="description"
      {...props}
      className={clsx(
        className,
        "col-span-2 col-start-2 row-start-2 text-sm/5 group-data-highlighted:text-white sm:text-xs/5 forced-colors:group-data-highlighted:text-[HighlightText] text-zinc-400"
      )}
    />
  );
}

export function DropdownShortcut({
  keys,
  className,
  ...props
}: { keys: string | string[]; className?: string } & React.ComponentProps<"kbd">) {
  return (
    <kbd
      {...props}
      className={clsx(
        "col-start-5 row-start-1 justify-self-end whitespace-pre font-sans text-zinc-400 group-data-highlighted:text-white forced-colors:group-data-highlighted:text-[HighlightText]",
        className
      )}
    >
      {Array.isArray(keys) ? keys.join(" ") : keys}
    </kbd>
  );
}

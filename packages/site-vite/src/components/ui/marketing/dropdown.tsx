import { Menu } from "@base-ui/react/menu";
import { cn } from "cn";
import type { ComponentProps } from "react";
import { Link } from "react-router";
import { Button } from "./button";

export const Dropdown = Menu.Root;

export function DropdownButton({
  render = <Button />,
  ...props
}: ComponentProps<typeof Menu.Trigger>) {
  return <Menu.Trigger render={render} {...props} />;
}

type DropdownMenuProps = Omit<ComponentProps<typeof Menu.Popup>, "className"> & {
  className?: string;
  align?: ComponentProps<typeof Menu.Positioner>["align"];
};

export function DropdownMenu({ align = "end", className, ...props }: DropdownMenuProps) {
  return (
    <Menu.Portal>
      <Menu.Positioner align={align} sideOffset={8} collisionPadding={8} className="z-50">
        <Menu.Popup
          {...props}
          className={cn(
            "max-h-(--available-height) min-w-48 max-w-(--available-width) overflow-y-auto overscroll-contain rounded-sm border border-white/10 bg-zinc-900 p-1 text-white shadow-xl outline-none transition-opacity duration-100 data-starting-style:opacity-0 data-ending-style:opacity-0 motion-reduce:transition-none",
            className
          )}
        />
      </Menu.Positioner>
    </Menu.Portal>
  );
}

type DropdownItemProps = Omit<ComponentProps<typeof Menu.Item>, "className"> & {
  href: string;
  className?: string;
};

export function DropdownItem({ href, className, ...props }: DropdownItemProps) {
  return (
    <Menu.Item
      {...props}
      render={<Link to={href} />}
      className={cn(
        "flex min-h-11 cursor-pointer touch-manipulation items-center rounded-sm px-3 py-2 text-sm/6 text-white outline-none data-highlighted:bg-orange-400 data-highlighted:text-zinc-950 forced-color-adjust-none forced-colors:data-highlighted:bg-[Highlight] forced-colors:data-highlighted:text-[HighlightText]",
        className
      )}
    />
  );
}

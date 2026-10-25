"use client";

import { Button as BaseButton } from "@base-ui/react/button";
import { Menu } from "@base-ui/react/menu";
import { cn } from "cn";
import type React from "react";
import {
  type ComponentPropsWithoutRef,
  createContext,
  createElement,
  type ElementType,
  useContext,
  useId,
  useLayoutEffect,
  useState,
} from "react";
import { Button } from "./button";
import { Link } from "./link";

const DescriptionContext = createContext<React.Dispatch<React.SetStateAction<string[]>> | null>(
  null
);

function useDescriptionId(id?: string) {
  const generatedId = useId();
  const descriptionId = id ?? generatedId;
  const setDescriptions = useContext(DescriptionContext);
  useLayoutEffect(() => {
    setDescriptions?.((ids) => [...ids, descriptionId]);
    return () => setDescriptions?.((ids) => ids.filter((value) => value !== descriptionId));
  }, [descriptionId, setDescriptions]);
  return descriptionId;
}

export function Dropdown(props: Menu.Root.Props) {
  return <Menu.Root loopFocus={false} {...props} />;
}

export function DropdownButton<T extends ElementType = typeof Button>({
  as,
  ...props
}: { as?: T } & ComponentPropsWithoutRef<T>) {
  return <Menu.Trigger disabled={props.disabled} render={createElement(as ?? Button, props)} />;
}

export function DropdownMenu({
  anchor = "bottom",
  className,
  ...props
}: { className?: string } & Omit<Menu.Popup.Props, "className"> & {
    anchor?: "top" | "bottom" | "top start" | "top end" | "bottom start" | "bottom end";
  }) {
  const [side, align = "center"] = anchor.split(" ") as [
    "top" | "bottom",
    ("start" | "center" | "end")?,
  ];
  return (
    <Menu.Portal>
      <Menu.Positioner
        side={side}
        align={align}
        sideOffset={8}
        alignOffset={() =>
          align === "center" ? 0 : window.matchMedia("(min-width: 40rem)").matches ? -4 : -6
        }
        collisionPadding={4}
        className="z-50"
      >
        <Menu.Popup
          {...props}
          className={cn(
            className,
            // Anchor positioning
            "max-h-(--available-height) max-w-(--available-width)",
            // Base styles
            "isolate w-max rounded-xl p-1",
            // Invisible border that is only visible in `forced-colors` mode for accessibility purposes
            "outline outline-transparent focus:outline-hidden",
            // Handle scrolling when menu won't fit in viewport
            "overflow-y-auto",
            // Popover background
            "backdrop-blur-xl bg-zinc-800/75",
            // Shadows
            "shadow-lg ring-1 ring-white/10 ring-inset",
            // Define grid at the menu level if subgrid is supported
            "supports-[grid-template-columns:subgrid]:grid supports-[grid-template-columns:subgrid]:grid-cols-[auto_1fr_1.5rem_0.5rem_auto]",
            // Transitions
            "transition data-ending-style:opacity-0 data-ending-style:duration-100 data-ending-style:ease-in"
          )}
        />
      </Menu.Positioner>
    </Menu.Portal>
  );
}

export function DropdownItem({
  className,
  ...props
}: { className?: string } & (
  | Omit<Menu.Item.Props, "className">
  | (Omit<React.ComponentPropsWithoutRef<typeof Link>, "className"> & { disabled?: boolean })
)) {
  const [descriptions, setDescriptions] = useState<string[]>([]);
  const classes = cn(
    className,
    // Base styles
    "group cursor-default rounded-lg px-3.5 py-2.5 focus:outline-hidden sm:px-3 sm:py-1.5",
    // Text styles
    "text-left text-base/6 sm:text-sm/6 text-white forced-colors:text-[CanvasText]",
    // Focus
    "data-highlighted:bg-blue-500 data-highlighted:text-white",
    // Disabled state
    "data-disabled:opacity-50",
    // Forced colors mode
    "forced-color-adjust-none forced-colors:data-highlighted:bg-[Highlight] forced-colors:data-highlighted:text-[HighlightText] forced-colors:data-highlighted:*:data-[slot=icon]:text-[HighlightText]",
    // Use subgrid when available but fallback to an explicit grid layout if not
    "col-span-full grid grid-cols-[auto_1fr_1.5rem_0.5rem_auto] items-center supports-[grid-template-columns:subgrid]:grid-cols-subgrid",
    // Icons
    "*:data-[slot=icon]:col-start-1 *:data-[slot=icon]:row-start-1 *:data-[slot=icon]:mr-2.5 *:data-[slot=icon]:-ml-0.5 *:data-[slot=icon]:size-5 sm:*:data-[slot=icon]:mr-2 sm:*:data-[slot=icon]:size-4",
    "data-highlighted:*:data-[slot=icon]:text-white *:data-[slot=icon]:text-zinc-400",
    // Avatar
    "*:data-[slot=avatar]:mr-2.5 *:data-[slot=avatar]:-ml-1 *:data-[slot=avatar]:size-6 sm:*:data-[slot=avatar]:mr-2 sm:*:data-[slot=avatar]:size-5"
  );

  return (
    <DescriptionContext value={setDescriptions}>
      {props.disabled ? (
        <BaseButton
          disabled
          role="menuitem"
          tabIndex={-1}
          id={props.id}
          title={props.title}
          aria-label={props["aria-label"]}
          aria-describedby={descriptions.join(" ") || undefined}
          className={classes}
        >
          {props.children}
        </BaseButton>
      ) : "href" in props ? (
        <Menu.Item
          disabled={props.disabled}
          aria-describedby={descriptions.join(" ") || undefined}
          render={<Link {...props} />}
          className={classes}
        />
      ) : (
        <Menu.Item
          aria-describedby={descriptions.join(" ") || undefined}
          {...props}
          nativeButton
          render={<button type="button" />}
          className={classes}
        />
      )}
    </DescriptionContext>
  );
}

export function DropdownHeader({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return <div {...props} className={cn(className, "col-span-5 px-3.5 pt-2.5 pb-1 sm:px-3")} />;
}

export function DropdownSection({
  className,
  ...props
}: { className?: string } & Omit<Menu.Group.Props, "as" | "className">) {
  return (
    <Menu.Group
      {...props}
      className={cn(
        className,
        // Define grid at the section level instead of the item level if subgrid is supported
        "col-span-full supports-[grid-template-columns:subgrid]:grid supports-[grid-template-columns:subgrid]:grid-cols-[auto_1fr_1.5rem_0.5rem_auto]"
      )}
    />
  );
}

export function DropdownHeading({
  className,
  ...props
}: { className?: string } & Omit<Menu.GroupLabel.Props, "as" | "className">) {
  return (
    <Menu.GroupLabel
      {...props}
      className={cn(
        className,
        "col-span-full grid grid-cols-[1fr_auto] gap-x-12 px-3.5 pt-2 pb-1 font-medium text-sm/5 sm:px-3 sm:text-xs/5 text-zinc-400"
      )}
    />
  );
}

export function DropdownDivider({
  className,
  ...props
}: { className?: string } & Omit<Menu.Separator.Props, "as" | "className">) {
  return (
    <Menu.Separator
      {...props}
      className={cn(
        className,
        "col-span-full mx-3.5 my-1 h-px border-0 sm:mx-3 bg-white/10 forced-colors:bg-[CanvasText]"
      )}
    />
  );
}

export function DropdownLabel({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return <div {...props} className={cn(className, "col-start-2 row-start-1")} data-slot="label" />;
}

export function DropdownDescription({
  className,
  ...props
}: { className?: string } & Omit<React.ComponentPropsWithoutRef<"p">, "as" | "className">) {
  const id = useDescriptionId(props.id);
  return (
    <p
      id={id}
      data-slot="description"
      {...props}
      className={cn(
        className,
        "col-span-2 col-start-2 row-start-2 text-sm/5 group-data-highlighted:text-white sm:text-xs/5 text-zinc-400 forced-colors:group-data-highlighted:text-[HighlightText]"
      )}
    />
  );
}

export function DropdownShortcut({
  keys,
  className,
  ...props
}: { keys: string | string[]; className?: string } & Omit<
  React.ComponentPropsWithoutRef<"kbd">,
  "as" | "className"
>) {
  const id = useDescriptionId(props.id);
  return (
    <kbd
      id={id}
      {...props}
      className={cn(className, "col-start-5 row-start-1 flex justify-self-end")}
    >
      {(Array.isArray(keys) ? keys : keys.split("")).map((char, index) => (
        <kbd
          className={cn([
            "min-w-[2ch] text-center font-sans text-zinc-400 capitalize group-data-highlighted:text-white forced-colors:group-data-highlighted:text-[HighlightText]",
            // Make sure key names that are longer than one character (like "Tab") have extra space
            index > 0 && char.length > 1 && "pl-1",
          ])}
          key={char}
        >
          {char}
        </kbd>
      ))}
    </kbd>
  );
}

import { Drawer as BaseDrawer } from "@base-ui/react/drawer";
import clsx from "clsx";
import type { ComponentProps } from "react";
import styles from "./drawer.module.css";
import { Text } from "./text";

const sizes = {
  xs: "max-w-xs",
  sm: "max-w-sm",
  md: "max-w-md",
  lg: "max-w-lg",
  xl: "max-w-xl",
  "2xl": "max-w-2xl",
  "3xl": "max-w-3xl",
  "4xl": "max-w-4xl",
  "5xl": "max-w-5xl",
};

const directions = { left: "left", right: "right", bottom: "down" } as const;

export function Drawer<Payload = unknown>({
  side = "right",
  ...props
}: Omit<BaseDrawer.Root.Props<Payload>, "swipeDirection"> & {
  side?: keyof typeof directions;
}) {
  return <BaseDrawer.Root {...props} swipeDirection={directions[side]} />;
}

export const DrawerTrigger = BaseDrawer.Trigger;
export const DrawerClose = BaseDrawer.Close;

export function DrawerPanel({
  size = "md",
  className,
  children,
  ...props
}: Omit<ComponentProps<typeof BaseDrawer.Popup>, "className"> & {
  size?: keyof typeof sizes;
  className?: string;
}) {
  return (
    <BaseDrawer.Portal>
      <BaseDrawer.Backdrop
        className={clsx(styles.backdrop, "pointer-events-none absolute inset-0 bg-zinc-950/50")}
      />
      <BaseDrawer.Viewport className="pointer-events-none fixed inset-0 overflow-hidden">
        <BaseDrawer.Popup
          {...props}
          className={clsx(
            styles.panel,
            "group/drawer pointer-events-auto flex min-h-0 flex-col bg-zinc-900 text-white shadow-xl ring-1 ring-white/10 scheme-dark outline-none forced-colors:outline",
            sizes[size],
            className
          )}
        >
          <div
            aria-hidden="true"
            className="hidden shrink-0 justify-center pt-3 pb-1 group-data-[swipe-direction=down]/drawer:flex"
          >
            <div className="h-1 w-10 rounded-full bg-zinc-600" />
          </div>
          <BaseDrawer.Content
            className={clsx(
              styles.content,
              "flex min-h-0 flex-1 flex-col p-(--gutter) [--gutter:--spacing(6)] sm:[--gutter:--spacing(8)]"
            )}
          >
            {children}
          </BaseDrawer.Content>
        </BaseDrawer.Popup>
      </BaseDrawer.Viewport>
    </BaseDrawer.Portal>
  );
}

export function DrawerTitle({
  className,
  ...props
}: Omit<ComponentProps<typeof BaseDrawer.Title>, "className"> & { className?: string }) {
  return (
    <BaseDrawer.Title
      {...props}
      className={clsx(
        "shrink-0 text-lg/6 font-semibold text-balance text-white sm:text-base/6",
        className
      )}
    />
  );
}

export function DrawerDescription({
  className,
  ...props
}: Omit<ComponentProps<typeof BaseDrawer.Description>, "className"> & { className?: string }) {
  return (
    <BaseDrawer.Description
      render={<Text />}
      {...props}
      className={clsx("mt-2 shrink-0 text-pretty", className)}
    />
  );
}

export function DrawerBody({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      {...props}
      className={clsx("mt-6 min-h-0 flex-1 overflow-y-auto overscroll-contain", className)}
    />
  );
}

export function DrawerActions({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      {...props}
      className={clsx(
        "mt-8 flex shrink-0 flex-col-reverse items-center justify-end gap-3 *:w-full sm:flex-row sm:*:w-auto",
        className
      )}
    />
  );
}

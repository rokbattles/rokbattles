import { Drawer as BaseDrawer } from "@base-ui/react/drawer";
import clsx from "clsx";
import type { ComponentProps } from "react";
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
      <BaseDrawer.Backdrop className="pointer-events-none absolute inset-0 bg-zinc-950/50 opacity-[calc(1-var(--drawer-swipe-progress,0))] transition-opacity duration-300 ease-[ease] data-starting-style:opacity-0 data-ending-style:opacity-0 data-ending-style:duration-[calc(var(--drawer-swipe-strength,1)*300ms)] data-swiping:duration-0 motion-reduce:transition-none" />
      <BaseDrawer.Viewport className="pointer-events-none fixed inset-0 overflow-hidden">
        <BaseDrawer.Popup
          {...props}
          className={clsx(
            "absolute w-[calc(100%-2.5rem)] transition-[transform,height] duration-300 ease-[ease] [--stack-depth:var(--nested-drawers,0)] [--stack-scale:max(0.8,calc(1-var(--stack-depth)*0.04))] [--stack-offset:calc(min(var(--stack-depth),5)*1rem)]",
            "data-[swipe-direction=right]:inset-y-0 data-[swipe-direction=right]:right-0 data-[swipe-direction=right]:rounded-l-2xl data-[swipe-direction=right]:origin-right data-[swipe-direction=right]:[transform:translateX(calc(var(--drawer-swipe-movement-x,0px)-var(--stack-offset)-(1-var(--stack-scale))*100%))_scale(var(--stack-scale))]",
            "data-[swipe-direction=left]:inset-y-0 data-[swipe-direction=left]:left-0 data-[swipe-direction=left]:rounded-r-2xl data-[swipe-direction=left]:origin-left data-[swipe-direction=left]:[transform:translateX(calc(var(--drawer-swipe-movement-x,0px)+var(--stack-offset)+(1-var(--stack-scale))*100%))_scale(var(--stack-scale))]",
            "data-[swipe-direction=down]:inset-x-0 data-[swipe-direction=down]:bottom-0 data-[swipe-direction=down]:mx-auto data-[swipe-direction=down]:w-full data-[swipe-direction=down]:h-[var(--drawer-height,auto)] data-[swipe-direction=down]:max-h-[calc(100%-2.5rem)] data-[swipe-direction=down]:rounded-t-3xl data-[swipe-direction=down]:origin-bottom data-[swipe-direction=down]:[transform:translateY(calc(var(--drawer-snap-point-offset,0px)+var(--drawer-swipe-movement-y,0px)-var(--stack-offset)-(1-var(--stack-scale))*100%))_scale(var(--stack-scale))]",
            "data-[swipe-direction=down]:data-nested-drawer-open:h-[var(--drawer-frontmost-height,var(--drawer-height,auto))]",
            "data-[swipe-direction=right]:data-starting-style:[transform:translateX(100%)] data-[swipe-direction=right]:data-ending-style:[transform:translateX(100%)] data-[swipe-direction=left]:data-starting-style:[transform:translateX(-100%)] data-[swipe-direction=left]:data-ending-style:[transform:translateX(-100%)] data-[swipe-direction=down]:data-starting-style:[transform:translateY(100%)] data-[swipe-direction=down]:data-ending-style:[transform:translateY(100%)]",
            "data-ending-style:duration-[calc(var(--drawer-swipe-strength,1)*300ms)] data-swiping:duration-0 data-nested-drawer-swiping:duration-0 motion-reduce:transition-none",
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
              "overflow-hidden pb-[max(var(--gutter),env(safe-area-inset-bottom))] transition-opacity duration-300 ease-[ease] group-data-nested-drawer-open/drawer:pointer-events-none group-data-nested-drawer-open/drawer:opacity-0 group-data-nested-drawer-swiping/drawer:opacity-100 motion-reduce:transition-none",
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

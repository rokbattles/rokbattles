import { AlertDialog as BaseDialog } from "@base-ui/react/alert-dialog";
import { cn } from "cn";
import type { ComponentProps, ReactNode } from "react";
import { Text } from "./text";

const sizes = {
  xs: "sm:max-w-xs",
  sm: "sm:max-w-sm",
  md: "sm:max-w-md",
  lg: "sm:max-w-lg",
  xl: "sm:max-w-xl",
  "2xl": "sm:max-w-2xl",
  "3xl": "sm:max-w-3xl",
  "4xl": "sm:max-w-4xl",
  "5xl": "sm:max-w-5xl",
};

type AlertProps = Omit<BaseDialog.Root.Props, "children"> & {
  size?: keyof typeof sizes;
  className?: string;
  children: ReactNode;
  initialFocus?: BaseDialog.Popup.Props["initialFocus"];
  finalFocus?: BaseDialog.Popup.Props["finalFocus"];
};

export function Alert({
  size = "md",
  className,
  children,
  initialFocus,
  finalFocus,
  ...props
}: AlertProps) {
  return (
    <BaseDialog.Root {...props}>
      <BaseDialog.Portal>
        <BaseDialog.Backdrop className="absolute inset-0 bg-zinc-950/50 transition-opacity duration-100 data-starting-style:opacity-0 data-ending-style:opacity-0 motion-reduce:transition-none" />
        <BaseDialog.Viewport className="fixed inset-0 overflow-y-auto overscroll-contain">
          <div className="grid min-h-full grid-rows-[1fr_auto_1fr] justify-items-center p-4 sm:grid-rows-[1fr_auto_3fr]">
            <BaseDialog.Popup
              initialFocus={initialFocus}
              finalFocus={finalFocus}
              className={cn(
                "row-start-2 w-full min-w-0 rounded-2xl bg-zinc-900 p-(--gutter) text-white shadow-lg ring-1 ring-white/10 scheme-dark [--gutter:--spacing(8)] forced-colors:outline",
                "transition duration-100 data-starting-style:scale-95 data-starting-style:opacity-0 data-ending-style:opacity-0 motion-reduce:transition-none",
                sizes[size],
                className
              )}
            >
              {children}
            </BaseDialog.Popup>
          </div>
        </BaseDialog.Viewport>
      </BaseDialog.Portal>
    </BaseDialog.Root>
  );
}

export const AlertClose = BaseDialog.Close;

export function AlertTitle({
  className,
  ...props
}: Omit<ComponentProps<typeof BaseDialog.Title>, "className"> & { className?: string }) {
  return (
    <BaseDialog.Title
      {...props}
      className={cn(
        "text-center text-base/6 font-semibold text-balance text-white sm:text-left sm:text-sm/6 sm:text-wrap",
        className
      )}
    />
  );
}

export function AlertDescription({
  className,
  ...props
}: Omit<ComponentProps<typeof BaseDialog.Description>, "className"> & { className?: string }) {
  return (
    <BaseDialog.Description
      render={<Text />}
      {...props}
      className={cn("mt-2 text-center text-pretty sm:text-left", className)}
    />
  );
}

export function AlertBody({ className, ...props }: ComponentProps<"div">) {
  return <div {...props} className={cn("mt-4", className)} />;
}

export function AlertActions({ className, ...props }: ComponentProps<"div">) {
  return (
    <div
      {...props}
      className={cn(
        "mt-6 flex flex-col-reverse items-center justify-end gap-3 *:w-full sm:flex-row sm:*:w-auto",
        className
      )}
    />
  );
}

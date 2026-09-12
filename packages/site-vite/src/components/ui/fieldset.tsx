import { Field as BaseField } from "@base-ui/react/field";
import { Fieldset as BaseFieldset } from "@base-ui/react/fieldset";
import clsx from "clsx";
import type React from "react";

export function Fieldset({
  className,
  ...props
}: { className?: string } & Omit<BaseFieldset.Root.Props, "className">) {
  return (
    <BaseFieldset.Root
      {...props}
      className={clsx(className, "*:data-[slot=text]:mt-1 [&>*+[data-slot=control]]:mt-6")}
    />
  );
}

export function Legend({
  className,
  ...props
}: { className?: string } & Omit<BaseFieldset.Legend.Props, "className">) {
  return (
    <BaseFieldset.Legend
      data-slot="legend"
      {...props}
      className={clsx(
        className,
        "text-base/6 font-semibold data-disabled:opacity-50 sm:text-sm/6 text-white"
      )}
    />
  );
}

export function FieldGroup({ className, ...props }: React.ComponentPropsWithoutRef<"div">) {
  return <div data-slot="control" {...props} className={clsx(className, "space-y-8")} />;
}

export function Field({
  className,
  ...props
}: { className?: string } & Omit<BaseField.Root.Props, "className">) {
  return (
    <BaseField.Root
      {...props}
      className={clsx(
        className,
        "[&>[data-slot=label]+[data-slot=control]]:mt-3",
        "[&>[data-slot=label]+[data-slot=description]]:mt-1",
        "[&>[data-slot=description]+[data-slot=control]]:mt-3",
        "[&>[data-slot=control]+[data-slot=description]]:mt-3",
        "[&>[data-slot=control]+[data-slot=error]]:mt-3",
        "*:data-[slot=label]:font-medium"
      )}
    />
  );
}

export function Label({
  className,
  ...props
}: { className?: string } & Omit<BaseField.Label.Props, "className">) {
  return (
    <BaseField.Label
      data-slot="label"
      {...props}
      className={clsx(
        className,
        "text-base/6 select-none data-disabled:opacity-50 sm:text-sm/6 text-white"
      )}
    />
  );
}

export function Description({
  className,
  ...props
}: { className?: string } & Omit<BaseField.Description.Props, "className">) {
  return (
    <BaseField.Description
      data-slot="description"
      {...props}
      className={clsx(className, "text-base/6 data-disabled:opacity-50 sm:text-sm/6 text-zinc-400")}
    />
  );
}

export function ErrorMessage({
  className,
  ...props
}: { className?: string } & Omit<BaseField.Error.Props, "className">) {
  return (
    <BaseField.Error
      data-slot="error"
      match={true}
      {...props}
      className={clsx(className, "text-base/6 data-disabled:opacity-50 sm:text-sm/6 text-red-500")}
    />
  );
}

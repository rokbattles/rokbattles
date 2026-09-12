import { cn } from "cn";

export function DescriptionList({ className, ...props }: React.ComponentPropsWithoutRef<"dl">) {
  return (
    <dl
      {...props}
      className={cn(
        className,
        "grid grid-cols-1 text-base/6 sm:grid-cols-[min(50%,--spacing(80))_auto] sm:text-sm/6"
      )}
    />
  );
}

export function DescriptionTerm({ className, ...props }: React.ComponentPropsWithoutRef<"dt">) {
  return (
    <dt
      {...props}
      className={cn(
        className,
        "col-start-1 border-t pt-3 first:border-none sm:border-t sm:py-3 border-white/5 text-zinc-400 sm:border-white/5"
      )}
    />
  );
}

export function DescriptionDetails({ className, ...props }: React.ComponentPropsWithoutRef<"dd">) {
  return (
    <dd
      {...props}
      className={cn(
        className,
        "pt-1 pb-3 sm:border-t sm:py-3 sm:nth-2:border-none text-white sm:border-white/5"
      )}
    />
  );
}

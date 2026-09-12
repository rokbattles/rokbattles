import { cn } from "cn";

export function Divider({
  soft = false,
  className,
  ...props
}: { soft?: boolean } & React.ComponentPropsWithoutRef<"hr">) {
  return (
    <hr
      role="presentation"
      {...props}
      className={cn(
        className,
        "w-full border-t",
        soft && "border-white/5",
        !soft && "border-white/10"
      )}
    />
  );
}

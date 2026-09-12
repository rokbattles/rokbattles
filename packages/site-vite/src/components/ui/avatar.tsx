import { Avatar as BaseAvatar } from "@base-ui/react/avatar";
import { Button as BaseButton } from "@base-ui/react/button";
import { cn } from "cn";
import type React from "react";
import { TouchTarget } from "./button";
import { Link } from "./link";

type AvatarProps = {
  src?: string | null;
  square?: boolean;
  initials?: string;
  alt?: string;
  className?: string;
};

export function Avatar({
  src = null,
  square = false,
  initials,
  alt = "",
  className,
  ...props
}: AvatarProps & React.ComponentPropsWithoutRef<"span">) {
  return (
    <BaseAvatar.Root
      data-slot="avatar"
      {...props}
      className={cn(
        className,
        "inline-grid shrink-0 align-middle [--avatar-radius:20%] *:col-start-1 *:row-start-1 outline -outline-offset-1 outline-white/10",
        square
          ? "rounded-(--avatar-radius) *:rounded-(--avatar-radius)"
          : "rounded-full *:rounded-full"
      )}
    >
      <BaseAvatar.Fallback
        role={alt ? "img" : undefined}
        aria-label={alt || undefined}
        className="size-full"
      >
        <svg
          className="size-full fill-current p-[5%] text-[48px] font-medium uppercase select-none"
          viewBox="0 0 100 100"
          aria-hidden="true"
        >
          <text
            x="50%"
            y="50%"
            alignmentBaseline="middle"
            dominantBaseline="middle"
            textAnchor="middle"
            dy=".125em"
          >
            {initials}
          </text>
        </svg>
      </BaseAvatar.Fallback>
      {src && <BaseAvatar.Image className="size-full object-cover" src={src} alt={alt} />}
    </BaseAvatar.Root>
  );
}

export function AvatarButton({
  ref,
  src,
  square = false,
  initials,
  alt,
  className,
  ...props
}: AvatarProps &
  (
    | ({ href?: never } & Omit<BaseButton.Props, "className">)
    | ({ href: string } & Omit<React.ComponentPropsWithoutRef<typeof Link>, "className">)
  ) & { ref?: React.Ref<HTMLAnchorElement | HTMLButtonElement> }) {
  const classes = cn(
    className,
    square ? "rounded-[20%]" : "rounded-full",
    "relative inline-grid focus:not-focus-visible:outline-hidden focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-blue-500"
  );

  return typeof props.href === "string" ? (
    <Link
      aria-label={alt || undefined}
      {...props}
      className={classes}
      ref={ref as React.Ref<HTMLAnchorElement>}
    >
      <TouchTarget>
        <Avatar src={src} square={square} initials={initials} alt={alt} />
      </TouchTarget>
    </Link>
  ) : (
    <BaseButton aria-label={alt || undefined} {...props} className={classes} ref={ref}>
      <TouchTarget>
        <Avatar src={src} square={square} initials={initials} alt={alt} />
      </TouchTarget>
    </BaseButton>
  );
}

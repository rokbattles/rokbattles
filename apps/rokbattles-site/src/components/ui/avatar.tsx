"use client";

import * as Headless from "@headlessui/react";
import type React from "react";
import { forwardRef } from "react";
import { TouchTarget } from "@/components/ui/button";
import { Link } from "@/components/ui/link";
import { cn } from "@/lib/cn";

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
    <span
      data-slot="avatar"
      {...props}
      className={cn(
        className,
        // Basic layout
        "inline-grid shrink-0 align-middle [--avatar-radius:20%] *:col-start-1 *:row-start-1",
        "outline -outline-offset-1 outline-black/10 dark:outline-white/10",
        // Border radius
        square
          ? "rounded-(--avatar-radius) *:rounded-(--avatar-radius)"
          : "rounded-full *:rounded-full"
      )}
    >
      {initials && (
        // biome-ignore lint/a11y/noSvgWithoutTitle: can safely ignore
        <svg
          className="size-full fill-current p-[5%] text-[48px] font-medium uppercase select-none"
          viewBox="0 0 100 100"
          aria-hidden={alt ? undefined : "true"}
        >
          {alt && <title>{alt}</title>}
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
      )}
      {/* biome-ignore lint/performance/noImgElement: can safely ignore */}
      {src && <img className="size-full" src={src} alt={alt} />}
    </span>
  );
}

type AvatarButtonLinkProps = AvatarProps &
  Omit<React.ComponentProps<typeof Link>, "className" | "children"> & { href: string };
type AvatarButtonProps = AvatarProps &
  Omit<Headless.ButtonProps, "as" | "className" | "children"> & { href?: never };

export const AvatarButton = forwardRef<HTMLElement, AvatarButtonLinkProps | AvatarButtonProps>(
  function AvatarButton(props, ref) {
    const { square = false, className } = props;

    const classes = cn(
      className,
      square ? "rounded-[20%]" : "rounded-full",
      "relative inline-grid focus:not-data-focus:outline-hidden data-focus:outline-2 data-focus:outline-offset-2 data-focus:outline-blue-500"
    );

    if ("href" in props && props.href) {
      const { href, src, initials, alt, ...rest } = props as AvatarButtonLinkProps;
      return (
        <Link
          {...rest}
          href={href}
          className={classes}
          ref={ref as React.ForwardedRef<HTMLAnchorElement>}
        >
          <TouchTarget>
            <Avatar src={src} square={square} initials={initials} alt={alt} />
          </TouchTarget>
        </Link>
      );
    }

    const { src, initials, alt, ...rest } = props as AvatarButtonProps;
    return (
      <Headless.Button {...rest} className={classes} ref={ref}>
        <TouchTarget>
          <Avatar src={src} square={square} initials={initials} alt={alt} />
        </TouchTarget>
      </Headless.Button>
    );
  }
);

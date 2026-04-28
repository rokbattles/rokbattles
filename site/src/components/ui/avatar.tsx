import {
  Button as HeadlessButton,
  type ButtonProps as HeadlessButtonProps,
} from "@headlessui/react";
import Image from "next/image";
import type React from "react";
import { forwardRef } from "react";
import { cn } from "@/lib/cn";
import { TouchTarget } from "./button";
import { Link } from "./link";

interface AvatarProps {
  src?: string | null;
  square?: boolean;
  initials?: string;
  alt?: string;
  className?: string;
}

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
        "relative inline-grid shrink-0 align-middle [--avatar-radius:20%] *:col-start-1 *:row-start-1",
        "outline outline-black/10 -outline-offset-1 dark:outline-white/10",
        // Border radius
        square
          ? "rounded-(--avatar-radius) *:rounded-(--avatar-radius)"
          : "rounded-full *:rounded-full"
      )}
    >
      {initials && (
        // biome-ignore lint/a11y/noSvgWithoutTitle: can safely ignore
        <svg
          aria-hidden={alt ? undefined : "true"}
          className="size-full select-none fill-current p-[5%] font-medium text-[48px] uppercase"
          viewBox="0 0 100 100"
        >
          {alt && <title>{alt}</title>}
          <text
            alignmentBaseline="middle"
            dominantBaseline="middle"
            dy=".125em"
            textAnchor="middle"
            x="50%"
            y="50%"
          >
            {initials}
          </text>
        </svg>
      )}
      {src && <Image alt={alt} className="object-cover" fill sizes="100%" src={src} />}
    </span>
  );
}

export const AvatarButton = forwardRef(function AvatarButton(
  {
    src,
    square = false,
    initials,
    alt,
    className,
    ...props
  }: AvatarProps &
    (
      | Omit<HeadlessButtonProps, "as" | "className">
      | Omit<React.ComponentPropsWithoutRef<typeof Link>, "className">
    ),
  ref: React.ForwardedRef<HTMLElement>
) {
  const classes = cn(
    className,
    square ? "rounded-[20%]" : "rounded-full",
    "relative inline-grid focus:not-data-focus:outline-hidden data-focus:outline-2 data-focus:outline-blue-500 data-focus:outline-offset-2"
  );

  return "href" in props ? (
    <Link {...props} className={classes} ref={ref as React.ForwardedRef<HTMLAnchorElement>}>
      <TouchTarget>
        <Avatar alt={alt} initials={initials} square={square} src={src} />
      </TouchTarget>
    </Link>
  ) : (
    <HeadlessButton {...props} className={classes} ref={ref}>
      <TouchTarget>
        <Avatar alt={alt} initials={initials} square={square} src={src} />
      </TouchTarget>
    </HeadlessButton>
  );
});

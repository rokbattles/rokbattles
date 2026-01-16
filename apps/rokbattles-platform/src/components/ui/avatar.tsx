import { type ButtonProps, Button as HeadlessButton } from "@headlessui/react";
import Image from "next/image";
import type React from "react";
import { forwardRef } from "react";
import { cn } from "@/lib/cn";
import { TouchTarget } from "./button";
import { Link } from "./link";

type AvatarProps = {
  src?: string | null;
  frameSrc?: string | null;
  square?: boolean;
  initials?: string;
  alt?: string;
  className?: string;
};

export function Avatar({
  src = null,
  frameSrc = null,
  square = false,
  initials,
  alt = "",
  className,
  ...props
}: AvatarProps & React.ComponentPropsWithoutRef<"span">) {
  const resolvedFrameSrc =
    typeof frameSrc === "string" && frameSrc.trim().length > 0
      ? frameSrc.trim()
      : null;
  const radiusClass = square ? "rounded-(--avatar-radius)" : "rounded-full";

  return (
    <span
      data-slot="avatar"
      {...props}
      className={cn(
        className,
        // Basic layout
        "inline-grid shrink-0 align-middle [--avatar-radius:20%] *:col-start-1 *:row-start-1",
        "outline outline-black/10 -outline-offset-1 dark:outline-white/10",
        "relative",
        // Border radius
        radiusClass
      )}
    >
      {initials && (
        // biome-ignore lint/a11y/noSvgWithoutTitle: false positive
        <svg
          aria-hidden={alt ? undefined : "true"}
          className={cn(
            "size-full select-none fill-current p-[5%] font-medium text-[48px] uppercase",
            radiusClass
          )}
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
      {src && (
        <Image
          alt={alt}
          className={cn("object-cover", radiusClass)}
          fill
          loading="lazy"
          src={src}
          unoptimized
        />
      )}
      {resolvedFrameSrc ? (
        <Image
          alt=""
          className="pointer-events-none z-10 scale-[1.15] rounded-none object-contain"
          fill
          loading="lazy"
          src={resolvedFrameSrc}
          unoptimized
        />
      ) : null}
    </span>
  );
}

export const AvatarButton = forwardRef(function AvatarButton(
  {
    src,
    frameSrc,
    square = false,
    initials,
    alt,
    className,
    ...props
  }: AvatarProps &
    (
      | ({ href?: never } & Omit<ButtonProps, "as" | "className">)
      | ({ href: string } & Omit<
          React.ComponentPropsWithoutRef<typeof Link>,
          "className"
        >)
    ),
  ref: React.ForwardedRef<HTMLButtonElement>
) {
  const classes = cn(
    className,
    square ? "rounded-[20%]" : "rounded-full",
    "relative inline-grid focus:not-data-focus:outline-hidden data-focus:outline-2 data-focus:outline-blue-500 data-focus:outline-offset-2"
  );

  return typeof props.href === "string" ? (
    // @ts-expect-error
    <Link
      {...props}
      className={classes}
      ref={ref as React.ForwardedRef<HTMLAnchorElement>}
    >
      <TouchTarget>
        <Avatar
          alt={alt}
          frameSrc={frameSrc}
          initials={initials}
          square={square}
          src={src}
        />
      </TouchTarget>
    </Link>
  ) : (
    // @ts-expect-error
    <HeadlessButton {...props} className={classes} ref={ref}>
      <TouchTarget>
        <Avatar
          alt={alt}
          frameSrc={frameSrc}
          initials={initials}
          square={square}
          src={src}
        />
      </TouchTarget>
    </HeadlessButton>
  );
});

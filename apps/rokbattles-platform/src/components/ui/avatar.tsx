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
    typeof frameSrc === "string" && frameSrc.trim().length > 0 ? frameSrc.trim() : null;
  const radiusClass = square ? "rounded-(--avatar-radius)" : "rounded-full";

  return (
    <span
      data-slot="avatar"
      {...props}
      className={cn(
        className,
        // Basic layout
        "inline-grid shrink-0 align-middle [--avatar-radius:20%] *:col-start-1 *:row-start-1",
        "outline -outline-offset-1 outline-black/10 dark:outline-white/10",
        "relative",
        // Border radius
        radiusClass
      )}
    >
      {initials && (
        // biome-ignore lint/a11y/noSvgWithoutTitle: false positive
        <svg
          className={cn(
            "size-full fill-current p-[5%] text-[48px] font-medium uppercase select-none",
            radiusClass
          )}
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
      {src && (
        <Image
          className={cn("object-cover", radiusClass)}
          src={src}
          alt={alt}
          fill
          unoptimized
          loading="lazy"
        />
      )}
      {resolvedFrameSrc ? (
        <Image
          className="pointer-events-none z-10 scale-[1.15] object-contain rounded-none"
          src={resolvedFrameSrc}
          alt=""
          fill
          unoptimized
          loading="lazy"
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
      | ({ href: string } & Omit<React.ComponentPropsWithoutRef<typeof Link>, "className">)
    ),
  ref: React.ForwardedRef<HTMLButtonElement>
) {
  const classes = cn(
    className,
    square ? "rounded-[20%]" : "rounded-full",
    "relative inline-grid focus:not-data-focus:outline-hidden data-focus:outline-2 data-focus:outline-offset-2 data-focus:outline-blue-500"
  );

  return typeof props.href === "string" ? (
    // @ts-expect-error
    <Link {...props} className={classes} ref={ref as React.ForwardedRef<HTMLAnchorElement>}>
      <TouchTarget>
        <Avatar src={src} frameSrc={frameSrc} square={square} initials={initials} alt={alt} />
      </TouchTarget>
    </Link>
  ) : (
    // @ts-expect-error
    <HeadlessButton {...props} className={classes} ref={ref}>
      <TouchTarget>
        <Avatar src={src} frameSrc={frameSrc} square={square} initials={initials} alt={alt} />
      </TouchTarget>
    </HeadlessButton>
  );
});

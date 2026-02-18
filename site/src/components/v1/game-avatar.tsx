import Image from "next/image";
import { cn } from "@/lib/cn";

interface GameAvatarProps {
  avatarUrl?: string | null;
  frameUrl?: string | null;
  square?: boolean;
  initials?: string;
  alt?: string;
  className?: string;
}

function normalize(input?: string) {
  try {
    const url = new URL(input);
    if (url.protocol === "http:") {
      url.protocol = "https:";
    }
    return url.href;
  } catch {
    return null;
  }
}

export function GameAvatar({
  avatarUrl,
  frameUrl,
  square = false,
  initials,
  alt = "",
  className,
  ...props
}: GameAvatarProps & React.ComponentPropsWithoutRef<"span">) {
  const resolvedAvatarUrl = normalize(avatarUrl);
  const resolvedFrameUrl = normalize(frameUrl);
  const radiusClass = square ? "rounded-(--avatar-radius)" : "rounded-full";

  return (
    <span
      data-slot="avatar"
      {...props}
      className={cn(
        className,
        "inline-grid shrink-0 align-middle [--avatar-radius:20%] *:col-start-1 *:row-start-1",
        "outline outline-black/10 -outline-offset-1 dark:outline-white/10",
        "relative",
        radiusClass
      )}
    >
      {initials ? (
        // biome-ignore lint/a11y/noSvgWithoutTitle: can safely ignore
        <svg
          aria-hidden={alt ? undefined : "true"}
          className={cn(
            "size-full select-none fill-current p-[5%] font-medium text-[48px] uppercase",
            radiusClass
          )}
          viewBox="0 0 100 100"
        >
          {alt ? <title>{alt}</title> : null}
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
      ) : null}
      {resolvedAvatarUrl ? (
        <Image
          alt={alt}
          className={cn("object-cover", radiusClass)}
          fill
          loading="lazy"
          src={resolvedAvatarUrl}
          unoptimized
        />
      ) : null}
      {resolvedFrameUrl ? (
        <Image
          alt=""
          className="pointer-events-none z-10 scale-[1.15] rounded-none object-contain"
          fill
          loading="lazy"
          src={resolvedFrameUrl}
          unoptimized
        />
      ) : null}
    </span>
  );
}

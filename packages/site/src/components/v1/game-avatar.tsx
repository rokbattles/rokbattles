import { cn } from "cnfast";
import Image from "next/image";

interface GameAvatarProps {
  avatarUrl?: string | null;
  frameUrl?: string | null;
  avatarOverride?: boolean;
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
  avatarOverride = false,
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
      {initials && !avatarOverride ? (
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
        <span
          className={cn("absolute", avatarOverride ? "z-20" : "inset-0")}
          style={
            avatarOverride
              ? {
                  height: "154.61%",
                  left: "-27.53%",
                  top: "-56.74%",
                  width: "154.61%",
                }
              : undefined
          }
        >
          <Image
            alt={alt}
            className={cn("object-cover", radiusClass)}
            fill
            loading="lazy"
            sizes="48px"
            src={resolvedAvatarUrl}
            unoptimized
          />
        </span>
      ) : null}
      {resolvedFrameUrl ? (
        <Image
          alt=""
          className="pointer-events-none z-10 scale-[1.15] rounded-none object-contain"
          fill
          loading="lazy"
          sizes="48px"
          src={resolvedFrameUrl}
          unoptimized
        />
      ) : null}
    </span>
  );
}

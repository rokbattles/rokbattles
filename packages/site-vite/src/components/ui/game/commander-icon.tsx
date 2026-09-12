import clsx from "clsx";

type CommanderIconProps = {
  sprites: string[];
  alt: string;
  className?: string;
};

export function CommanderIcon({ sprites, alt, className }: CommanderIconProps) {
  if (sprites.length === 0) {
    return null;
  }

  return (
    <span
      aria-label={alt}
      className={clsx("relative inline-grid size-8 shrink-0 align-middle", className)}
      role="img"
    >
      {sprites.map((sprite) => (
        <img
          key={sprite}
          alt=""
          className="absolute inset-0 size-full object-contain"
          loading="lazy"
          src={sprite}
        />
      ))}
    </span>
  );
}

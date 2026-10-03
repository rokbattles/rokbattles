export const frame =
  "mx-auto w-[calc(100%-2rem)] max-w-7xl border-x border-white/10 sm:w-[calc(100%-4rem)]";
export const gutter = "px-6 sm:px-8";
export const sectionHeading = "px-6 py-10 sm:px-8 sm:py-12";
export const card = "bg-zinc-950 p-6 sm:p-8";

export function GridMarkers() {
  return (
    <span
      aria-hidden="true"
      className="pointer-events-none absolute inset-x-0 top-0 z-10 font-mono text-xl/none text-zinc-400"
    >
      <span className="absolute left-0 -translate-x-1/2 -translate-y-1/2">+</span>
      <span className="absolute right-0 translate-x-1/2 -translate-y-1/2">+</span>
    </span>
  );
}

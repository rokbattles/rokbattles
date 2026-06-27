import type { ReactNode } from "react";

type WatchedDirectoriesProps = {
  dirs: string[];
  isLoading: boolean;
  onRemove: (dir: string) => void;
};

export function WatchedDirectories({
  dirs,
  isLoading,
  onRemove,
}: WatchedDirectoriesProps): ReactNode {
  const hasDirs = dirs.length > 0;

  return (
    <section className="mt-4 border-t border-white/10">
      {isLoading ? (
        <div className="py-6 text-sm text-zinc-400">Loading directories...</div>
      ) : hasDirs ? (
        <ul className="divide-y divide-white/10">
          {dirs.map((dir) => (
            <li key={dir} className="flex items-center justify-between gap-4 py-3">
              <div className="min-w-0">
                <p className="truncate text-sm/6 font-medium text-zinc-200">{dir}</p>
              </div>
              <button
                type="button"
                onClick={() => onRemove(dir)}
                className="rounded-lg px-2 py-1 text-sm/5 font-medium text-zinc-400 hover:bg-white/5 hover:text-white"
              >
                Remove
              </button>
            </li>
          ))}
        </ul>
      ) : (
        <div className="py-6 text-sm text-zinc-400">No directories are being watched.</div>
      )}
    </section>
  );
}

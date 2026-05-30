type WatchedDirectoriesCardProps = {
  dirs: string[];
  isLoading: boolean;
  onRemove: (dir: string) => void;
};

export function WatchedDirectories({ dirs, isLoading, onRemove }: WatchedDirectoriesCardProps) {
  const hasDirs = dirs.length > 0;

  return (
    <section className="rounded-lg border border-zinc-800 bg-zinc-900/60 backdrop-blur-sm">
      <div className="flex items-center justify-between border-b border-zinc-800 p-4">
        <h2 className="text-base font-medium">Watched Directories</h2>
      </div>
      {isLoading ? (
        <div className="p-8 text-center text-sm text-zinc-400">Loading directories...</div>
      ) : hasDirs ? (
        <ul className="divide-y divide-zinc-800">
          {dirs.map((dir) => (
            <li key={dir} className="flex items-center justify-between gap-4 p-3">
              <div className="min-w-0">
                <p className="truncate text-sm font-medium text-zinc-200">{dir}</p>
              </div>
              <button
                type="button"
                onClick={() => onRemove(dir)}
                className="inline-flex items-center rounded-md border border-zinc-700 bg-zinc-800 px-2.5 py-1.5 text-xs font-medium text-zinc-200 hover:bg-zinc-700"
              >
                Remove
              </button>
            </li>
          ))}
        </ul>
      ) : (
        <div className="p-8 text-center text-sm text-zinc-400">
          Click "Add directory" to start watching a directory.
        </div>
      )}
    </section>
  );
}

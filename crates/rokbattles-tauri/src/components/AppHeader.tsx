type AppHeaderProps = {
  isAdding: boolean;
  isReprocessing: boolean;
  onAdd: () => void;
  onReprocess: () => void;
};

export function AppHeader({ isAdding, isReprocessing, onAdd, onReprocess }: AppHeaderProps) {
  return (
    <header className="mb-6 flex items-center justify-between gap-3">
      <h1 className="text-xl font-semibold tracking-tight">ROK Battles</h1>
      <div className="flex items-center gap-2">
        <button
          type="button"
          onClick={onReprocess}
          disabled={isReprocessing}
          className="inline-flex items-center gap-2 rounded-md bg-zinc-700 px-3 py-2 text-sm font-medium text-white hover:bg-zinc-600 disabled:opacity-60"
        >
          {isReprocessing ? "Reprocessing..." : "Reprocess all"}
        </button>
        <button
          type="button"
          onClick={onAdd}
          disabled={isAdding || isReprocessing}
          className="inline-flex items-center gap-2 rounded-md bg-zinc-700 px-3 py-2 text-sm font-medium text-white hover:bg-zinc-600 disabled:opacity-60"
        >
          {isAdding ? "Adding..." : "Add directory"}
        </button>
      </div>
    </header>
  );
}

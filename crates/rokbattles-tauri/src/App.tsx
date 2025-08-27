import "./App.css";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useMemo, useState } from "react";

function App() {
  const [dirs, setDirs] = useState<string[]>([]);
  const [isAdding, setIsAdding] = useState(false);
  const [isLoading, setIsLoading] = useState(true);

  const refresh = async () => {
    try {
      setIsLoading(true);
      const list = (await invoke<string[]>("list_dirs")) ?? [];
      setDirs(Array.isArray(list) ? list : []);
    } catch (e) {
      console.error("Failed to list watched dirs", e);
      setDirs([]);
    } finally {
      setIsLoading(false);
    }
  };

  // biome-ignore lint/correctness/useExhaustiveDependencies: incorrect lint error
  useEffect(() => {
    refresh();
  }, []);

  const hasDirs = dirs.length > 0;
  const heading = useMemo(
    () => (isLoading ? "Loading..." : hasDirs ? "Watched directories" : "No directories yet"),
    [hasDirs, isLoading]
  );

  const handleAdd = async () => {
    try {
      setIsAdding(true);
      const selection = await open({
        multiple: true,
        directory: true,
      });
      if (!selection) return;

      const selected = Array.isArray(selection) ? selection : [selection];

      await invoke("add_dir", { paths: selected });
      await refresh();
    } catch (e) {
      console.error("Failed to add dirs", e);
    } finally {
      setIsAdding(false);
    }
  };

  const handleRemove = async (dir: string) => {
    try {
      await invoke("remove_dir", { path: dir });
      await refresh();
    } catch (e) {
      console.error("Failed to remove dir", e);
    }
  };

  return (
    <main className="min-h-screen bg-zinc-950 text-zinc-100">
      <div className="mx-auto max-w-3xl p-6">
        <header className="mb-6 flex items-center justify-between">
          <h1 className="text-xl font-semibold tracking-tight">ROK Battles</h1>
          <button
            type="button"
            onClick={handleAdd}
            disabled={isAdding}
            className="inline-flex items-center gap-2 rounded-md bg-zinc-700 px-3 py-2 text-sm font-medium text-white hover:bg-zinc-600 disabled:opacity-60"
          >
            {isAdding ? "Adding..." : "Add directory"}
          </button>
        </header>
        <section className="rounded-lg border border-zinc-800 bg-zinc-900/60 backdrop-blur-sm">
          <div className="flex items-center justify-between border-b border-zinc-800 p-4">
            <h2 className="text-base font-medium">{heading}</h2>
            {!isLoading && hasDirs && <span className="text-xs text-zinc-400">{dirs.length}</span>}
          </div>
          {isLoading ? (
            <div className="p-8 text-center text-sm text-zinc-400">Loading directories...</div>
          ) : hasDirs ? (
            <ul className="divide-y divide-zinc-800">
              {dirs.map((dir) => (
                <li key={dir} className="flex items-center justify-between gap-4 p-4">
                  <div className="min-w-0">
                    <p className="truncate text-sm font-medium text-zinc-200">{dir}</p>
                  </div>
                  <button
                    type="button"
                    onClick={() => handleRemove(dir)}
                    className="inline-flex items-center rounded-md border border-zinc-700 bg-zinc-800 px-2.5 py-1.5 text-xs font-medium text-zinc-200 hover:bg-zinc-700"
                  >
                    Remove
                  </button>
                </li>
              ))}
            </ul>
          ) : (
            <div className="p-8 text-center text-sm text-zinc-400">
              Click "Add directory" to start watching folders.
            </div>
          )}
        </section>
      </div>
    </main>
  );
}

export default App;

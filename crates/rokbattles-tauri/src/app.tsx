import "./App.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { useEffect, useState } from "react";

const hasMessagePayload = (value: unknown): value is { message: unknown } =>
  typeof value === "object" && value !== null && "message" in value;

function App() {
  const [dirs, setDirs] = useState<string[]>([]);
  const [isAdding, setIsAdding] = useState(false);
  const [isReprocessing, setIsReprocessing] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [logs, setLogs] = useState<string[]>([]);

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

  useEffect(() => {
    refresh();
  }, []);

  useEffect(() => {
    let isMounted = true;
    const unlisten = listen<unknown>("rokbattles", (e) => {
      const payload = e.payload;
      const msg = hasMessagePayload(payload)
        ? String(payload.message)
        : String(payload);
      if (!isMounted) {
        return;
      }
      setLogs((prev) => {
        const next = [...prev, msg];
        return next.length > 100 ? next.slice(next.length - 100) : next;
      });
    });

    return () => {
      isMounted = false;
      unlisten.then((fn) => fn()).catch(console.error);
    };
  }, []);

  const hasDirs = dirs.length > 0;

  const handleAdd = async () => {
    try {
      setIsAdding(true);
      const selection = await open({
        multiple: true,
        directory: true,
      });
      if (!selection) {
        return;
      }

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

  const handleReprocess = async () => {
    try {
      setIsReprocessing(true);
      await invoke("reprocess_all");
    } catch (e) {
      console.error("Failed to trigger reprocess", e);
    } finally {
      setIsReprocessing(false);
    }
  };

  return (
    <main className="min-h-screen bg-zinc-950 text-zinc-100">
      <div className="mx-auto max-w-3xl p-6">
        <header className="mb-6 flex items-center justify-between gap-3">
          <h1 className="font-semibold text-xl tracking-tight">ROK Battles</h1>
          <div className="flex items-center gap-2">
            <button
              className="inline-flex items-center gap-2 rounded-md bg-zinc-700 px-3 py-2 font-medium text-sm text-white hover:bg-zinc-600 disabled:opacity-60"
              disabled={isReprocessing}
              onClick={handleReprocess}
              type="button"
            >
              {isReprocessing ? "Reprocessing..." : "Reprocess all"}
            </button>
            <button
              className="inline-flex items-center gap-2 rounded-md bg-zinc-700 px-3 py-2 font-medium text-sm text-white hover:bg-zinc-600 disabled:opacity-60"
              disabled={isAdding}
              onClick={handleAdd}
              type="button"
            >
              {isAdding ? "Adding..." : "Add directory"}
            </button>
          </div>
        </header>
        <section className="rounded-lg border border-zinc-800 bg-zinc-900/60 backdrop-blur-sm">
          <div className="flex items-center justify-between border-zinc-800 border-b p-4">
            <h2 className="font-medium text-base">Watched Directories</h2>
          </div>
          {isLoading && (
            <div className="p-8 text-center text-sm text-zinc-400">
              Loading directories...
            </div>
          )}
          {!isLoading && hasDirs ? (
            <ul className="divide-y divide-zinc-800">
              {dirs.map((dir) => (
                <li
                  className="flex items-center justify-between gap-4 p-3"
                  key={dir}
                >
                  <div className="min-w-0">
                    <p className="truncate font-medium text-sm text-zinc-200">
                      {dir}
                    </p>
                  </div>
                  <button
                    className="inline-flex items-center rounded-md border border-zinc-700 bg-zinc-800 px-2.5 py-1.5 font-medium text-xs text-zinc-200 hover:bg-zinc-700"
                    onClick={() => handleRemove(dir)}
                    type="button"
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
        <section className="mt-6 rounded-lg border border-zinc-800 bg-zinc-900/60 backdrop-blur-sm">
          <div className="flex items-center justify-between border-zinc-800 border-b p-4">
            <h2 className="font-medium text-base">Logs</h2>
          </div>
          <div className="h-64 overflow-y-auto overflow-x-hidden">
            {logs.length === 0 ? (
              <div className="h-full p-8 text-center text-sm text-zinc-400">
                No logs yet.
              </div>
            ) : (
              <div className="divide-y-0">
                {logs.map((log) => (
                  <div
                    className="whitespace-pre-wrap break-words px-3 py-1 font-mono text-xs text-zinc-300"
                    key={log}
                  >
                    {log}
                  </div>
                ))}
              </div>
            )}
          </div>
        </section>
      </div>
    </main>
  );
}

export default App;

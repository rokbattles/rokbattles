import { type ReactNode, useState } from "react";

import type { LogEntry } from "../lib/log-entry.ts";

type LogsProps = {
  watcherLogs: LogEntry[];
  networkLogs: LogEntry[];
};

type LogTab = "watcher" | "network";

const logTabs: Array<{ id: LogTab; label: string }> = [
  { id: "watcher", label: "Watcher" },
  { id: "network", label: "Network" },
];

export function Logs({ watcherLogs, networkLogs }: LogsProps): ReactNode {
  const [activeTab, setActiveTab] = useState<LogTab>("watcher");
  const logs = activeTab === "watcher" ? watcherLogs : networkLogs;

  return (
    <section className="mt-6 rounded-lg border border-zinc-800 bg-zinc-900/60 backdrop-blur-sm">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-zinc-800 p-4">
        <h2 className="text-base font-medium">Logs</h2>
        <div className="inline-flex rounded-md border border-zinc-800 bg-zinc-950 p-0.5">
          {logTabs.map((tab) => {
            const count = tab.id === "watcher" ? watcherLogs.length : networkLogs.length;
            const isActive = activeTab === tab.id;

            return (
              <button
                type="button"
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={`rounded px-2.5 py-1 text-xs font-medium ${
                  isActive ? "bg-zinc-700 text-zinc-100" : "text-zinc-400 hover:text-zinc-200"
                }`}
              >
                {tab.label}
                <span className="ml-1 text-zinc-500">{count}</span>
              </button>
            );
          })}
        </div>
      </div>
      <div className="h-64 overflow-y-auto overflow-x-hidden">
        {logs.length === 0 ? (
          <div className="h-full p-8 text-center text-sm text-zinc-400">No logs yet.</div>
        ) : (
          <div className="divide-y-0">
            {logs.map((log) => (
              <div
                key={log.id}
                className="px-3 py-1 font-mono text-xs text-zinc-300 break-words whitespace-pre-wrap"
              >
                {log.message}
              </div>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}

import type { ReactNode } from "react";

import type { LogEntry } from "../lib/log-entry.ts";

type LogsProps = {
  logs: LogEntry[];
};

export function Logs({ logs }: LogsProps): ReactNode {
  return (
    <section className="mt-6 rounded-lg border border-zinc-800 bg-zinc-900/60 backdrop-blur-sm">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-zinc-800 p-4">
        <h2 className="text-base font-medium">Logs</h2>
        <span className="rounded-md bg-zinc-800 px-2.5 py-1 text-xs font-medium text-zinc-400">
          {logs.length}
        </span>
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

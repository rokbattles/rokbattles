import type { ReactNode } from "react";

import type { LogEntry } from "../lib/log-entry.ts";

type LogsProps = {
  logs: LogEntry[];
};

export function Logs({ logs }: LogsProps): ReactNode {
  return (
    <section className="mt-8">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="text-base/6 font-semibold text-white">Logs</h2>
        <span className="rounded-md bg-white/5 px-1.5 py-0.5 text-xs/5 font-medium text-zinc-400">
          {logs.length}
        </span>
      </div>
      <div className="mt-4 h-64 overflow-y-auto overflow-x-hidden border-t border-white/10">
        {logs.length === 0 ? (
          <div className="py-6 text-sm text-zinc-400">No logs yet.</div>
        ) : (
          <div>
            {logs.map((log) => (
              <div
                key={log.id}
                className="py-1 font-mono text-xs text-zinc-300 break-words whitespace-pre-wrap"
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

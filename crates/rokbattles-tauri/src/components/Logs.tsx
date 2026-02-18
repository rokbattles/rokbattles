type LogsCardProps = {
  logs: string[];
};

export function Logs({ logs }: LogsCardProps) {
  return (
    <section className="mt-6 rounded-lg border border-zinc-800 bg-zinc-900/60 backdrop-blur-sm">
      <div className="flex items-center justify-between border-b border-zinc-800 p-4">
        <h2 className="text-base font-medium">Logs</h2>
      </div>
      <div className="h-64 overflow-y-auto overflow-x-hidden">
        {logs.length === 0 ? (
          <div className="h-full p-8 text-center text-sm text-zinc-400">No logs yet.</div>
        ) : (
          <div className="divide-y-0">
            {logs.map((log, idx) => (
              <div
                // biome-ignore lint/suspicious/noArrayIndexKey: logs do not have stable IDs from the backend.
                key={idx}
                className="px-3 py-1 font-mono text-xs text-zinc-300 break-words whitespace-pre-wrap"
              >
                {log}
              </div>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}

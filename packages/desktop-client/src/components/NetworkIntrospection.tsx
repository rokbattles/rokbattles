import type { NetworkStatus } from "../lib/tauri-client";

type NetworkIntrospectionProps = {
  status: NetworkStatus;
};

const statusLabels: Record<NetworkStatus["state"], string> = {
  disabled: "Disabled",
  waiting: "Waiting",
  connected: "Connected",
  disconnected: "Disconnected",
  error: "Error",
};

const statusClasses: Record<NetworkStatus["state"], string> = {
  disabled: "bg-zinc-700 text-zinc-200",
  waiting: "bg-amber-500/15 text-amber-200",
  connected: "bg-emerald-500/15 text-emerald-200",
  disconnected: "bg-sky-500/15 text-sky-200",
  error: "bg-rose-500/15 text-rose-200",
};

export function NetworkIntrospection({ status }: NetworkIntrospectionProps) {
  return (
    <section className="mt-6 rounded-lg border border-zinc-800 bg-zinc-900/60">
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-zinc-800 p-4">
        <h2 className="text-base font-medium">Network Introspection</h2>
        <span className="rounded-md bg-zinc-800 px-2.5 py-1 text-xs font-medium text-zinc-300">
          Experimental
        </span>
      </div>
      <div className="p-4">
        <div
          className={`inline-flex rounded-md px-2.5 py-1 text-sm font-medium ${statusClasses[status.state]}`}
        >
          {statusLabels[status.state]}
        </div>
        {status.message ? <p className="mt-2 text-sm text-zinc-400">{status.message}</p> : null}
      </div>
    </section>
  );
}

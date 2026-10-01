import { useEffect, useState } from "react";

import {
  getWorkerStatus,
  pauseWatcher,
  resumeWatcher,
  setCaptureOptIn,
  stopBackgroundWorker,
  type WorkerSnapshot,
} from "../lib/tauri-client.ts";
import { Button } from "./Button.tsx";

export function WorkerStatus() {
  const [snapshot, setSnapshot] = useState<WorkerSnapshot | null>(null);
  const [error, setError] = useState("");
  const [pending, setPending] = useState(false);

  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    const refresh = async () => {
      try {
        const next = await getWorkerStatus();
        if (active) setSnapshot(next);
      } catch {
        if (active)
          setError("Background state is unavailable. Reinstall the complete app if this persists.");
      } finally {
        if (active) timer = setTimeout(refresh, 1000);
      }
    };
    void refresh();
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, []);

  const act = async (action: () => Promise<unknown>) => {
    if (pending) return;
    setPending(true);
    setError("");
    try {
      await action();
      setSnapshot(await getWorkerStatus());
    } catch {
      setError(
        "The background worker could not apply that change. Check the installation and try again."
      );
    } finally {
      setPending(false);
    }
  };

  const paused = snapshot?.status.paused ?? false;
  const state = !snapshot
    ? "Loading"
    : snapshot.maintenance
      ? "Update pending"
      : !snapshot.enabled
        ? "Stopped"
        : !snapshot.alive
          ? "Unavailable"
          : paused
            ? "Paused"
            : "Mailcache active";

  const captureState = snapshot?.status.capture_state ?? 0;
  const captureBackend = snapshot?.status.capture_backend === 1 ? "WinDivert" : "pcap";
  const captureLabel =
    !snapshot?.alive || !snapshot.capture_opt_in || paused
      ? "Off"
      : ([
          "Off",
          "Connecting to helper",
          `Waiting for a new game connection (${captureBackend})`,
          `Streaming server traffic (${captureBackend})`,
          "Helper or native capture unavailable",
          "Capture interrupted; reconnect the game",
          "Platform capture setup or worker restart required",
        ][captureState] ?? "Unavailable");

  return (
    <section
      className="mb-6 rounded-xl border border-zinc-700 p-4 text-sm text-zinc-300"
      aria-label="Background worker"
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="font-semibold text-white">Background worker: {state}</h2>
        <div className="flex gap-2">
          <Button
            variant="outline"
            disabled={pending || !snapshot}
            onClick={() => void act(paused || !snapshot?.alive ? resumeWatcher : pauseWatcher)}
          >
            {paused || !snapshot?.alive ? "Start / resume" : "Pause"}
          </Button>
          <Button
            variant="outline"
            disabled={pending || !snapshot?.enabled}
            onClick={() => void act(stopBackgroundWorker)}
          >
            Stop background worker
          </Button>
        </div>
      </div>
      <p className="mt-2">
        Closing this window or quitting the UI leaves the worker running. Use Pause or Stop here to
        stop uploads.
      </p>
      <p className="mt-2">
        Mailcache pending: {snapshot?.status.pending ?? 0} · Completed:{" "}
        {snapshot?.status.completed ?? 0} · Unsupported/rejected: {snapshot?.status.rejected ?? 0}
      </p>
      <label className="mt-4 flex items-start gap-2">
        <input
          type="checkbox"
          checked={snapshot?.capture_opt_in ?? false}
          disabled={pending || !snapshot}
          onChange={(event) => void act(() => setCaptureOptIn(event.target.checked))}
        />
        <span>Allow network capture when the capture helper is available</span>
      </label>
      <p className="mt-2 text-xs/5 text-zinc-400">
        Capture sends server-to-game traffic on ports 3101 and 5222 to ROK Battles for continuous
        mail decoding. Other server messages are discarded there. Client payloads are excluded;
        zero-payload connection controls stay on this computer. Raw streams are not saved. Capture
        requires the platform helper and permissions. Windows Npcap is optional, installed
        separately by you, and never bundled. Mailcache remains available without native capture.
      </p>
      <p className="mt-2 text-xs/5 text-zinc-400">
        Network capture: {captureLabel}. After capture loss, reconnect the game. Earlier mails may
        already have been stored; interrupted streams are never replayed. If the helper is
        unavailable, complete platform capture setup or continue using mailcache. Previous JSON
        history is not imported; reselect your directories once.
      </p>
      {snapshot?.maintenance ? (
        <p className="mt-2 text-amber-300">
          Complete the installer or repair an interrupted update before restarting the worker. Your
          enabled setting is preserved.
        </p>
      ) : null}
      {error ? (
        <p role="alert" className="mt-2 text-amber-300">
          {error}
        </p>
      ) : null}
    </section>
  );
}

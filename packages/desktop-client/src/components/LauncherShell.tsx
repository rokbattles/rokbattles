import { getCurrentWindow } from "@tauri-apps/api/window";
import { type ReactNode, useEffect, useRef, useState } from "react";

import { useAppVersion } from "../hooks/useAppVersion.ts";
import { useCaptureStatus } from "../hooks/useCaptureStatus.ts";
import { useWindowMaximized } from "../hooks/useWindowMaximized.ts";
import {
  type CaptureStatus,
  closeMainWindow,
  type LauncherView,
  openFeedback,
  selectView,
} from "../lib/tauri-client.ts";

const captureLabels: Record<CaptureStatus, { text: string; tone: string }> = {
  active: { text: "Watcher active", tone: "green" },
  paused: { text: "Watcher paused", tone: "orange" },
  not_configured: { text: "Watcher not configured", tone: "red" },
  starting: { text: "Watcher starting", tone: "orange" },
  stopping: { text: "Watcher stopping", tone: "orange" },
  updating: { text: "Watcher paused · Updating", tone: "orange" },
  exiting: { text: "Watcher stopping", tone: "orange" },
  failed: { text: "Watcher unavailable", tone: "red" },
};

export function LauncherShell(): ReactNode {
  const version = useAppVersion();
  const captureStatus = useCaptureStatus();
  const maximized = useWindowMaximized();

  const [view, setView] = useState<LauncherView>("desktop");
  const [pending, setPending] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const switching = useRef(false);

  const capture = captureStatus
    ? captureLabels[captureStatus]
    : { text: "Checking watcher…", tone: "muted" };

  useEffect(() => {
    let disposed = false;

    selectView("desktop")
      .catch((cause) => {
        console.error("Failed to load Desktop", cause);
        if (!disposed) setError("Could not load Desktop. Select the tab to retry.");
      })
      .finally(() => {
        if (!disposed) setPending(false);
      });

    return () => {
      disposed = true;
    };
  }, []);

  async function switchView(next: LauncherView) {
    if (pending || switching.current) return;

    switching.current = true;
    setPending(true);
    setError(null);

    try {
      await selectView(next);
      setView(next);
    } catch (cause) {
      console.error("Failed to switch view", cause);
      setError(`Could not open ${next === "app" ? "App" : "Desktop"}. Select the tab to retry.`);
    } finally {
      switching.current = false;
      setPending(false);
    }
  }

  async function windowAction(action: () => Promise<unknown>) {
    try {
      await action();
    } catch (cause) {
      console.error("Window action failed", cause);
      setError("That action could not be completed. Please try again.");
    }
  }

  return (
    <div className="launcher-shell">
      <header className="launcher-titlebar" data-tauri-drag-region>
        <img
          className="launcher-wordmark"
          src="/rokbattles-wordmark-white.svg"
          alt="ROK Battles"
          width="144"
          height="16"
          draggable={false}
        />

        <nav aria-label="Main navigation" className="launcher-tabs">
          {(["desktop", "app"] as const).map((tab) => (
            <button
              key={tab}
              type="button"
              className="launcher-tab"
              aria-current={view === tab ? "page" : undefined}
              disabled={pending}
              onClick={() => void switchView(tab)}
            >
              {tab === "desktop" ? "Desktop" : "App"}
            </button>
          ))}
        </nav>

        <div className="launcher-drag-region" data-tauri-drag-region />

        <div className="launcher-window-controls">
          <button
            type="button"
            aria-label="Minimize"
            title="Minimize"
            onClick={() => void windowAction(() => getCurrentWindow().minimize())}
          >
            <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
              <path d="M1 6h10" />
            </svg>
          </button>

          <button
            type="button"
            aria-label={maximized ? "Restore" : "Maximize"}
            title={maximized ? "Restore" : "Maximize"}
            onClick={() => void windowAction(() => getCurrentWindow().toggleMaximize())}
          >
            <svg width="12" height="12" viewBox="0 0 12 12" fill="none" aria-hidden="true">
              {maximized ? (
                <>
                  <path d="M4 4V1.5h6.5V8H8" />
                  <rect x="1.5" y="4" width="6.5" height="6.5" />
                </>
              ) : (
                <rect x="2" y="2" width="8" height="8" />
              )}
            </svg>
          </button>

          <button
            type="button"
            className="launcher-close"
            aria-label="Close"
            title="Close"
            onClick={() => void windowAction(closeMainWindow)}
          >
            <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
              <path d="m2 2 8 8M10 2l-8 8" />
            </svg>
          </button>
        </div>
      </header>

      <main className="launcher-content" aria-label={`${view === "app" ? "App" : "Desktop"} view`}>
        <span className="launcher-loading">{pending ? "Opening…" : ""}</span>
      </main>

      <footer className="launcher-footer">
        <span className="launcher-capture" data-tone={capture.tone} role="status">
          <span className="launcher-status-dot" aria-hidden="true" />
          <span>{capture.text}</span>
        </span>

        {error ? (
          <span className="launcher-error" role="alert" title={error}>
            {error}
          </span>
        ) : null}

        <div className="launcher-footer-actions">
          {version ? (
            <>
              <span className="launcher-version">v{version}</span>
              <span className="launcher-footer-separator" aria-hidden="true">
                ·
              </span>
            </>
          ) : null}

          <button
            type="button"
            className="launcher-feedback"
            onClick={() => void windowAction(openFeedback)}
          >
            Give feedback
          </button>
        </div>
      </footer>
    </div>
  );
}

import { getVersion } from "@tauri-apps/api/app";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import type { Dispatch, SetStateAction } from "react";
import { useCallback, useEffect, useRef, useState } from "react";

import { AppHeader } from "./components/AppHeader";
import { ClosePrompt } from "./components/ClosePrompt";
import { Logs } from "./components/Logs.tsx";
import { NetworkIntrospection } from "./components/NetworkIntrospection.tsx";
import { WatchedDirectories } from "./components/WatchedDirectories.tsx";
import { type CloseChoice, parseCloseBehavior } from "./lib/close-behavior";
import {
  addDirs,
  discoverMailcacheDirs,
  getCloseBehavior,
  getExperimentalNetworkIntrospection,
  getNetworkIntrospectionStatus,
  listDirs,
  minimizeToTray,
  type NetworkStatus,
  pauseWatcher,
  removeDir,
  reprocessAll,
  requestAppQuit,
  resumeWatcher,
  setCloseBehavior,
} from "./lib/tauri-client";

const appWindow = getCurrentWindow();

type BannerType = "success" | "info" | "error";

const bannerClasses: Record<BannerType, string> = {
  success: "border-emerald-700 bg-emerald-950/70 text-emerald-200",
  info: "border-sky-700 bg-sky-950/70 text-sky-200",
  error: "border-rose-700 bg-rose-950/70 text-rose-200",
};

const disabledNetworkStatus: NetworkStatus = {
  state: "disabled",
  message: "Network introspection is disabled.",
};

function App() {
  const [dirs, setDirs] = useState<string[]>([]);
  const [isAdding, setIsAdding] = useState(false);
  const [isDiscovering, setIsDiscovering] = useState(false);
  const [isReprocessing, setIsReprocessing] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [watcherLogs, setWatcherLogs] = useState<string[]>([]);
  const [networkLogs, setNetworkLogs] = useState<string[]>([]);
  const [banner, setBanner] = useState<{ type: BannerType; message: string } | null>(null);
  const [showClosePrompt, setShowClosePrompt] = useState(false);
  const [rememberCloseChoice, setRememberCloseChoice] = useState(false);
  const [isApplyingCloseChoice, setIsApplyingCloseChoice] = useState(false);
  const [appVersion, setAppVersion] = useState<string | null>(null);
  const [networkEnabled, setNetworkEnabled] = useState(false);
  const [networkStatus, setNetworkStatus] = useState<NetworkStatus>(disabledNetworkStatus);

  const allowCloseRef = useRef(false);
  const handlingCloseIntentRef = useRef(false);
  const closePromptOpenRef = useRef(false);
  const bannerTimerRef = useRef<number | null>(null);

  useEffect(() => {
    closePromptOpenRef.current = showClosePrompt;
  }, [showClosePrompt]);

  const refresh = useCallback(async () => {
    try {
      setIsLoading(true);
      const list = (await listDirs()) ?? [];
      setDirs(Array.isArray(list) ? list : []);
    } catch (error) {
      console.error("Failed to list watched dirs", error);
      setDirs([]);
    } finally {
      setIsLoading(false);
    }
  }, []);

  const showTransientBanner = useCallback((type: BannerType, message: string) => {
    setBanner({ type, message });

    if (bannerTimerRef.current !== null) {
      window.clearTimeout(bannerTimerRef.current);
    }

    bannerTimerRef.current = window.setTimeout(() => {
      setBanner(null);
      bannerTimerRef.current = null;
    }, 3000);
  }, []);

  const applyCloseChoice = useCallback(async (choice: CloseChoice, remember: boolean) => {
    try {
      setIsApplyingCloseChoice(true);

      if (remember) {
        await setCloseBehavior(choice);
      }

      if (choice === "minimize_to_tray") {
        await minimizeToTray();
      } else {
        allowCloseRef.current = true;
        await requestAppQuit();
      }
    } catch (error) {
      console.error("Failed to apply close behavior", error);
      allowCloseRef.current = false;
    } finally {
      setIsApplyingCloseChoice(false);
      setShowClosePrompt(false);
    }
  }, []);

  const handleCloseIntent = useCallback(async () => {
    if (allowCloseRef.current || handlingCloseIntentRef.current || closePromptOpenRef.current) {
      return;
    }

    handlingCloseIntentRef.current = true;
    try {
      const behavior = parseCloseBehavior(await getCloseBehavior());

      if (behavior === "ask") {
        setRememberCloseChoice(false);
        setShowClosePrompt(true);
        return;
      }

      await applyCloseChoice(behavior, false);
    } catch (error) {
      console.error("Failed to resolve close behavior", error);
      setRememberCloseChoice(false);
      setShowClosePrompt(true);
    } finally {
      handlingCloseIntentRef.current = false;
    }
  }, [applyCloseChoice]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    let isMounted = true;

    Promise.all([getExperimentalNetworkIntrospection(), getNetworkIntrospectionStatus()])
      .then(([enabled, status]) => {
        if (!isMounted) {
          return;
        }
        setNetworkEnabled(Boolean(enabled));
        setNetworkStatus(enabled ? status : disabledNetworkStatus);
      })
      .catch((error) => {
        console.error("Failed to load network introspection status", error);
      });

    return () => {
      isMounted = false;
    };
  }, []);

  useEffect(() => {
    let isMounted = true;

    getVersion()
      .then((version) => {
        if (isMounted) {
          setAppVersion(version);
        }
      })
      .catch((error) => {
        console.error("Failed to load app version", error);
      });

    return () => {
      isMounted = false;
    };
  }, []);

  useEffect(() => {
    return () => {
      if (bannerTimerRef.current !== null) {
        window.clearTimeout(bannerTimerRef.current);
      }
    };
  }, []);

  useEffect(() => {
    let isMounted = true;

    const unlisten = listen<{ message: string }>("rokbattles", (event) => {
      if (!isMounted) {
        return;
      }

      appendLog(setWatcherLogs, logMessage(event.payload));
    });

    return () => {
      isMounted = false;
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, []);

  useEffect(() => {
    let isMounted = true;

    const unlisten = listen<{ message: string }>("network-introspection-log", (event) => {
      if (!isMounted) {
        return;
      }

      appendLog(setNetworkLogs, logMessage(event.payload));
    });

    return () => {
      isMounted = false;
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, []);

  useEffect(() => {
    let isMounted = true;

    const unlisten = listen<NetworkStatus>("network-introspection", (event) => {
      if (!isMounted) {
        return;
      }
      setNetworkStatus(event.payload);
    });

    return () => {
      isMounted = false;
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, []);

  useEffect(() => {
    const unlistenClose = appWindow.onCloseRequested((event) => {
      if (allowCloseRef.current) {
        return;
      }

      event.preventDefault();
      void handleCloseIntent();
    });

    return () => {
      unlistenClose.then((fn) => fn()).catch(() => {});
    };
  }, [handleCloseIntent]);

  const handleAdd = useCallback(async () => {
    try {
      setIsAdding(true);
      await pauseWatcher();

      const selection = await open({
        multiple: true,
        directory: true,
      });
      if (!selection) {
        return;
      }

      const selected = Array.isArray(selection) ? selection : [selection];
      await addDirs(selected);
      await refresh();
    } catch (error) {
      console.error("Failed to add dirs", error);
    } finally {
      setIsAdding(false);
      await resumeWatcher();
    }
  }, [refresh]);

  const handleRemove = useCallback(
    async (dir: string) => {
      try {
        await removeDir(dir);
        await refresh();
      } catch (error) {
        console.error("Failed to remove dir", error);
      }
    },
    [refresh]
  );

  const handleDiscover = useCallback(async () => {
    try {
      setIsDiscovering(true);
      await pauseWatcher();

      const result = await discoverMailcacheDirs();
      await refresh();

      if (result.added_dirs.length > 0) {
        showTransientBanner("success", result.message);
        return;
      }
      if (result.already_watched_dirs.length > 0) {
        showTransientBanner("info", result.message);
        return;
      }

      showTransientBanner("error", result.message);
    } catch (error) {
      console.error("Failed to auto-discover mailcache dirs", error);
      showTransientBanner("error", "No valid mailcache directories were found.");
    } finally {
      setIsDiscovering(false);
      await resumeWatcher();
    }
  }, [refresh, showTransientBanner]);

  const handleReprocess = useCallback(async () => {
    try {
      setIsReprocessing(true);
      await reprocessAll();
    } catch (error) {
      console.error("Failed to trigger reprocess", error);
    } finally {
      setIsReprocessing(false);
    }
  }, []);

  const handlePromptChoice = useCallback(
    (choice: CloseChoice) => {
      void applyCloseChoice(choice, rememberCloseChoice);
    },
    [applyCloseChoice, rememberCloseChoice]
  );

  return (
    <main className="min-h-screen bg-zinc-950 text-zinc-100">
      <div className="mx-auto max-w-3xl p-6">
        <AppHeader
          isAdding={isAdding}
          isDiscovering={isDiscovering}
          isReprocessing={isReprocessing}
          onAdd={handleAdd}
          onDiscover={handleDiscover}
          onReprocess={handleReprocess}
        />
        {banner ? (
          <div className={`mb-4 rounded-md border px-3 py-2 text-sm ${bannerClasses[banner.type]}`}>
            {banner.message}
          </div>
        ) : null}
        <WatchedDirectories dirs={dirs} isLoading={isLoading} onRemove={handleRemove} />
        {networkEnabled ? <NetworkIntrospection status={networkStatus} /> : null}
        <Logs watcherLogs={watcherLogs} networkLogs={networkLogs} />
        {appVersion ? (
          <footer className="pt-4 text-center text-xs text-zinc-700">Version {appVersion}</footer>
        ) : null}
      </div>
      <ClosePrompt
        isOpen={showClosePrompt}
        rememberChoice={rememberCloseChoice}
        isApplying={isApplyingCloseChoice}
        onRememberChoiceChange={setRememberCloseChoice}
        onCancel={() => setShowClosePrompt(false)}
        onChoose={handlePromptChoice}
      />
    </main>
  );
}

function logMessage(payload: unknown): string {
  return payload && typeof payload === "object" && "message" in payload
    ? String(payload.message)
    : String(payload);
}

function appendLog(setLogs: Dispatch<SetStateAction<string[]>>, message: string) {
  setLogs((prev) => {
    const next = [...prev, message];
    return next.length > 100 ? next.slice(next.length - 100) : next;
  });
}

export default App;

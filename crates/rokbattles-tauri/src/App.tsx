import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useRef, useState } from "react";

import { AppHeader } from "./components/AppHeader";
import { ClosePrompt } from "./components/ClosePrompt";
import { Logs } from "./components/Logs.tsx";
import { WatchedDirectories } from "./components/WatchedDirectories.tsx";
import { type CloseChoice, parseCloseBehavior } from "./lib/close-behavior";
import {
  addDirs,
  getCloseBehavior,
  listDirs,
  minimizeToTray,
  pauseWatcher,
  removeDir,
  reprocessAll,
  requestAppQuit,
  resumeWatcher,
  setCloseBehavior,
} from "./lib/tauri-client";

const appWindow = getCurrentWindow();

function App() {
  const [dirs, setDirs] = useState<string[]>([]);
  const [isAdding, setIsAdding] = useState(false);
  const [isReprocessing, setIsReprocessing] = useState(false);
  const [isLoading, setIsLoading] = useState(true);
  const [logs, setLogs] = useState<string[]>([]);
  const [showClosePrompt, setShowClosePrompt] = useState(false);
  const [rememberCloseChoice, setRememberCloseChoice] = useState(false);
  const [isApplyingCloseChoice, setIsApplyingCloseChoice] = useState(false);

  const allowCloseRef = useRef(false);
  const handlingCloseIntentRef = useRef(false);
  const closePromptOpenRef = useRef(false);

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

    const unlisten = listen<{ message: string }>("rokbattles", (event) => {
      const payload = event.payload;
      const msg =
        payload && typeof payload === "object" && "message" in payload
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
          isReprocessing={isReprocessing}
          onAdd={handleAdd}
          onReprocess={handleReprocess}
        />
        <WatchedDirectories dirs={dirs} isLoading={isLoading} onRemove={handleRemove} />
        <Logs logs={logs} />
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

export default App;

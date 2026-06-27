import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useRef } from "react";

import { parseCloseBehavior } from "../lib/close-behavior.ts";
import { getCloseBehavior, minimizeToTray, requestAppQuit } from "../lib/tauri-client.ts";

const appWindow = getCurrentWindow();

export function useCloseBehavior(): void {
  const allowCloseRef = useRef(false);
  const handlingCloseIntentRef = useRef(false);

  const applyCloseBehavior = useCallback(async () => {
    try {
      const behavior = parseCloseBehavior(await getCloseBehavior());

      if (behavior === "minimize_to_tray") {
        await minimizeToTray();
      } else {
        allowCloseRef.current = true;
        await requestAppQuit();
      }
    } catch (error) {
      console.error("Failed to apply close behavior", error);
      allowCloseRef.current = false;
    }
  }, []);

  const handleCloseIntent = useCallback(async () => {
    if (allowCloseRef.current || handlingCloseIntentRef.current) {
      return;
    }

    handlingCloseIntentRef.current = true;
    try {
      await applyCloseBehavior();
    } finally {
      handlingCloseIntentRef.current = false;
    }
  }, [applyCloseBehavior]);

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
}

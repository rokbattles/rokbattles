import { getCurrentWindow } from "@tauri-apps/api/window";
import { useCallback, useEffect, useRef, useState } from "react";

import { type CloseChoice, parseCloseBehavior } from "../lib/close-behavior.ts";
import {
  getCloseBehavior,
  minimizeToTray,
  requestAppQuit,
  setCloseBehavior,
} from "../lib/tauri-client.ts";

const appWindow = getCurrentWindow();

type UseClosePromptResult = {
  isOpen: boolean;
  rememberChoice: boolean;
  isApplying: boolean;
  setRememberChoice: (remember: boolean) => void;
  cancel: () => void;
  choose: (choice: CloseChoice) => void;
};

export function useClosePrompt(): UseClosePromptResult {
  const [isOpen, setIsOpen] = useState(false);
  const [rememberChoice, setRememberChoice] = useState(false);
  const [isApplying, setIsApplying] = useState(false);

  const allowCloseRef = useRef(false);
  const handlingCloseIntentRef = useRef(false);

  const applyCloseChoice = useCallback(async (choice: CloseChoice, remember: boolean) => {
    try {
      setIsApplying(true);

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
      setIsApplying(false);
      setIsOpen(false);
    }
  }, []);

  const handleCloseIntent = useCallback(async () => {
    if (allowCloseRef.current || handlingCloseIntentRef.current || isOpen) {
      return;
    }

    handlingCloseIntentRef.current = true;
    try {
      const behavior = parseCloseBehavior(await getCloseBehavior());

      if (behavior === "ask") {
        setRememberChoice(false);
        setIsOpen(true);
        return;
      }

      await applyCloseChoice(behavior, false);
    } catch (error) {
      console.error("Failed to resolve close behavior", error);
      setRememberChoice(false);
      setIsOpen(true);
    } finally {
      handlingCloseIntentRef.current = false;
    }
  }, [applyCloseChoice, isOpen]);

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

  const choose = useCallback(
    (choice: CloseChoice) => {
      void applyCloseChoice(choice, rememberChoice);
    },
    [applyCloseChoice, rememberChoice]
  );

  const cancel = useCallback(() => {
    setIsOpen(false);
  }, []);

  return {
    isOpen,
    rememberChoice,
    isApplying,
    setRememberChoice,
    cancel,
    choose,
  };
}

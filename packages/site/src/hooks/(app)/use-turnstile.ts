"use client";

import { useEffect, useRef, useState } from "react";
import type { TurnstileOptions } from "@/lib/(app)/types/turnstile";

type TurnstileSettings = {
  siteKey: string | undefined;
  action: string;
  onVerify: (token: string) => void;
};

export function useTurnstile({ siteKey, action, onVerify }: TurnstileSettings) {
  const container = useRef<HTMLDivElement>(null);
  const widget = useRef<string | null>(null);
  const [ready, setReady] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const api = window.turnstile;

    if (!ready || !siteKey || !container.current || !api) return;

    function reject(message: string) {
      onVerify("");
      setError(message);
    }

    const options: TurnstileOptions = {
      sitekey: siteKey,
      action,
      theme: "dark",
      size: "flexible",
      callback: (token) => {
        setError(null);
        onVerify(token);
      },
      "expired-callback": () => reject("Verification expired. Please retry."),
      "error-callback": () => reject("Verification failed. Please retry."),
      "timeout-callback": () => reject("Verification timed out. Please retry."),
      "unsupported-callback": () =>
        reject("Verification is not supported in this browser. Please try another browser."),
    };

    const id = api.render(container.current, options);
    widget.current = id;

    // Remove the widget on close and during Strict Mode's effect cleanup.
    return () => {
      api.remove(id);
      widget.current = null;
    };
  }, [ready, siteKey, action, onVerify]);

  function retry() {
    onVerify("");
    setError(null);

    if (widget.current !== null) window.turnstile?.reset(widget.current);
  }

  function onReady() {
    setReady(true);
  }

  function onLoadError() {
    onVerify("");
    setError("Verification could not load. Refresh the page to try again.");
  }

  return { container, ready, error, retry, onReady, onLoadError };
}

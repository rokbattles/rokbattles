import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";

import type { LogEntry } from "../lib/log-entry.ts";

const maxLogEntries = 100;

type LogEventPayload = {
  message: string;
};

export function useLogEvents(eventName: string): LogEntry[] {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const nextIdRef = useRef(0);

  useEffect(() => {
    let isMounted = true;

    const unlisten = listen<LogEventPayload>(eventName, (event) => {
      if (!isMounted) {
        return;
      }

      const nextId = nextIdRef.current;
      nextIdRef.current += 1;

      setLogs((previousLogs) => {
        const nextLogs = [
          ...previousLogs,
          {
            id: `${eventName}-${nextId}`,
            message: logMessage(event.payload),
          },
        ];

        return nextLogs.length > maxLogEntries
          ? nextLogs.slice(nextLogs.length - maxLogEntries)
          : nextLogs;
      });
    });

    return () => {
      isMounted = false;
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, [eventName]);

  return logs;
}

function logMessage(payload: unknown): string {
  return payload && typeof payload === "object" && "message" in payload
    ? String(payload.message)
    : String(payload);
}

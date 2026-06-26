import { listen } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

import {
  getExperimentalNetworkIntrospection,
  getNetworkIntrospectionStatus,
  type NetworkStatus,
} from "../lib/tauri-client.ts";

const disabledNetworkStatus: NetworkStatus = {
  state: "disabled",
  message: "Network introspection is disabled.",
};

type UseNetworkIntrospectionResult = {
  isEnabled: boolean;
  status: NetworkStatus;
};

export function useNetworkIntrospection(): UseNetworkIntrospectionResult {
  const [isEnabled, setIsEnabled] = useState(false);
  const [status, setStatus] = useState<NetworkStatus>(disabledNetworkStatus);

  useEffect(() => {
    let isMounted = true;

    Promise.all([getExperimentalNetworkIntrospection(), getNetworkIntrospectionStatus()])
      .then(([enabled, nextStatus]) => {
        if (!isMounted) {
          return;
        }

        setIsEnabled(Boolean(enabled));
        setStatus(enabled ? nextStatus : disabledNetworkStatus);
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

    const unlisten = listen<NetworkStatus>("network-introspection", (event) => {
      if (isMounted) {
        setStatus(event.payload);
      }
    });

    return () => {
      isMounted = false;
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, []);

  return { isEnabled, status };
}

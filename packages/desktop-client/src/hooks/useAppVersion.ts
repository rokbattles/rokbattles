import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState } from "react";

export function useAppVersion(): string | null {
  const [appVersion, setAppVersion] = useState<string | null>(null);

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

  return appVersion;
}

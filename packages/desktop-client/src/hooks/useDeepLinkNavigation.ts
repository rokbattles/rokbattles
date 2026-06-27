import { listen } from "@tauri-apps/api/event";
import { useEffect } from "react";
import { useNavigate } from "react-router";

import { getCurrentDeepLinks } from "../lib/tauri-client.ts";

const deepLinkEventName = "deep-link://new-url";

function routePathFromDeepLink(value: unknown): string | null {
  if (typeof value !== "string") {
    return null;
  }

  let url: URL;
  try {
    url = new URL(value);
  } catch {
    return null;
  }

  if (url.protocol !== "rokbattles:") {
    return null;
  }

  const target = url.hostname || url.pathname.replace(/^\/+/, "");
  if (target === "" || target === "start") {
    return "/";
  }

  if (target === "settings") {
    return "/settings";
  }

  return null;
}

function routePathFromDeepLinks(urls: unknown[]): string | null {
  for (const url of urls) {
    const routePath = routePathFromDeepLink(url);
    if (routePath) {
      return routePath;
    }
  }

  return null;
}

export function useDeepLinkNavigation(): void {
  const navigate = useNavigate();

  useEffect(() => {
    const navigateToDeepLink = (urls: unknown[]) => {
      const routePath = routePathFromDeepLinks(urls);
      if (routePath) {
        navigate(routePath, { replace: true });
      }
    };

    void getCurrentDeepLinks()
      .then(navigateToDeepLink)
      .catch((error) => {
        console.error("Failed to read startup deep links", error);
      });

    const unlisten = listen<unknown[]>(deepLinkEventName, (event) => {
      navigateToDeepLink(Array.isArray(event.payload) ? event.payload : []);
    });

    return () => {
      unlisten.then((fn) => fn()).catch(() => {});
    };
  }, [navigate]);
}

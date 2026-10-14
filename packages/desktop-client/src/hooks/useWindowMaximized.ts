import { getCurrentWindow } from "@tauri-apps/api/window";
import { useEffect, useState } from "react";

export function useWindowMaximized(): boolean {
  const [maximized, setMaximized] = useState(false);

  useEffect(() => {
    const window = getCurrentWindow();
    let disposed = false;
    let revision = 0;

    const refresh = async () => {
      const current = ++revision;

      try {
        const next = await window.isMaximized();
        if (!disposed && current === revision) setMaximized(next);
      } catch (error) {
        console.error("Failed to read window state", error);
      }
    };

    // Also follows titlebar double-clicks and changes made by the window manager.
    const subscription = window.onResized(refresh);
    subscription.then(refresh).catch((error) => {
      console.error("Failed to listen for window resizing", error);
      void refresh();
    });

    return () => {
      disposed = true;
      subscription.then((unlisten) => unlisten()).catch(() => {});
    };
  }, []);

  return maximized;
}

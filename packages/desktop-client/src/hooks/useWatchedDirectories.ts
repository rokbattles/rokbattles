import { open } from "@tauri-apps/plugin-dialog";
import { useCallback, useEffect, useReducer } from "react";

import type { BannerType } from "../lib/banner.ts";
import {
  addDirs,
  discoverMailcacheDirs,
  listDirs,
  pauseWatcher,
  removeDir,
  reprocessAll,
  resumeWatcher,
} from "../lib/tauri-client.ts";

type ShowTransientBanner = (type: BannerType, message: string) => void;

type WatchedDirectoriesState = {
  dirs: string[];
  isAdding: boolean;
  isDiscovering: boolean;
  isReprocessing: boolean;
  isLoading: boolean;
};

type WatchedDirectoriesAction =
  | { type: "loadingStarted" }
  | { type: "dirsLoaded"; dirs: string[] }
  | { type: "dirsLoadFailed" }
  | { type: "addStarted" }
  | { type: "addFinished" }
  | { type: "discoverStarted" }
  | { type: "discoverFinished" }
  | { type: "reprocessStarted" }
  | { type: "reprocessFinished" };

type UseWatchedDirectoriesResult = WatchedDirectoriesState & {
  handleAdd: () => Promise<void>;
  handleDiscover: () => Promise<void>;
  handleRemove: (dir: string) => Promise<void>;
  handleReprocess: () => Promise<void>;
};

const initialWatchedDirectoriesState: WatchedDirectoriesState = {
  dirs: [],
  isAdding: false,
  isDiscovering: false,
  isReprocessing: false,
  isLoading: true,
};

export function useWatchedDirectories(
  showTransientBanner: ShowTransientBanner
): UseWatchedDirectoriesResult {
  const [state, dispatch] = useReducer(watchedDirectoriesReducer, initialWatchedDirectoriesState);

  const refresh = useCallback(async () => {
    try {
      dispatch({ type: "loadingStarted" });
      const dirs = (await listDirs()) ?? [];
      dispatch({ type: "dirsLoaded", dirs: Array.isArray(dirs) ? dirs : [] });
    } catch (error) {
      console.error("Failed to list watched dirs", error);
      dispatch({ type: "dirsLoadFailed" });
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const handleAdd = useCallback(async () => {
    try {
      dispatch({ type: "addStarted" });
      await pauseWatcher();

      const selection = await open({
        multiple: true,
        directory: true,
      });
      if (!selection) {
        return;
      }

      const selectedDirs = Array.isArray(selection) ? selection : [selection];
      await addDirs(selectedDirs);
      await refresh();
    } catch (error) {
      console.error("Failed to add dirs", error);
    } finally {
      dispatch({ type: "addFinished" });
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
      dispatch({ type: "discoverStarted" });
      await pauseWatcher();

      const result = await discoverMailcacheDirs();
      let bannerType: BannerType = "error";

      if (result.added_dirs.length > 0) {
        bannerType = "success";
      } else if (result.already_watched_dirs.length > 0) {
        bannerType = "info";
      }

      await refresh();
      showTransientBanner(bannerType, result.message);
    } catch (error) {
      console.error("Failed to auto-discover mailcache dirs", error);
      showTransientBanner("error", "No valid mailcache directories were found.");
    } finally {
      dispatch({ type: "discoverFinished" });
      await resumeWatcher();
    }
  }, [refresh, showTransientBanner]);

  const handleReprocess = useCallback(async () => {
    try {
      dispatch({ type: "reprocessStarted" });
      await reprocessAll();
    } catch (error) {
      console.error("Failed to trigger reprocess", error);
    } finally {
      dispatch({ type: "reprocessFinished" });
    }
  }, []);

  return {
    ...state,
    handleAdd,
    handleDiscover,
    handleRemove,
    handleReprocess,
  };
}

function watchedDirectoriesReducer(
  state: WatchedDirectoriesState,
  action: WatchedDirectoriesAction
): WatchedDirectoriesState {
  switch (action.type) {
    case "loadingStarted":
      return { ...state, isLoading: true };
    case "dirsLoaded":
      return { ...state, dirs: action.dirs, isLoading: false };
    case "dirsLoadFailed":
      return { ...state, dirs: [], isLoading: false };
    case "addStarted":
      return { ...state, isAdding: true };
    case "addFinished":
      return { ...state, isAdding: false };
    case "discoverStarted":
      return { ...state, isDiscovering: true };
    case "discoverFinished":
      return { ...state, isDiscovering: false };
    case "reprocessStarted":
      return { ...state, isReprocessing: true };
    case "reprocessFinished":
      return { ...state, isReprocessing: false };
  }
}

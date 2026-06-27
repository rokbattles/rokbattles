import { invoke } from "@tauri-apps/api/core";

import type { CloseBehavior, CloseChoice } from "./close-behavior.ts";

export type DiscoverMailcacheResult = {
  added_dirs: string[];
  already_watched_dirs: string[];
  message: string;
};

export function listDirs(): Promise<string[]> {
  return invoke<string[]>("list_dirs");
}

export function addDirs(paths: string[]): Promise<string[]> {
  return invoke<string[]>("add_dir", { paths });
}

export function removeDir(path: string): Promise<string[]> {
  return invoke<string[]>("remove_dir", { path });
}

export function discoverMailcacheDirs(): Promise<DiscoverMailcacheResult> {
  return invoke<DiscoverMailcacheResult>("discover_mailcache_dirs");
}

export function pauseWatcher(): Promise<unknown> {
  return invoke("pause_watcher");
}

export function resumeWatcher(): Promise<unknown> {
  return invoke("resume_watcher");
}

export function reprocessAll(): Promise<unknown> {
  return invoke("reprocess_all");
}

export function getCloseBehavior(): Promise<CloseBehavior> {
  return invoke<CloseBehavior>("get_close_behavior");
}

export function setCloseBehavior(behavior: CloseChoice): Promise<unknown> {
  return invoke("set_close_behavior", { behavior });
}

export function minimizeToTray(): Promise<unknown> {
  return invoke("minimize_to_tray");
}

export function requestAppQuit(): Promise<unknown> {
  return invoke("request_app_quit");
}

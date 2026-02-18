import { invoke } from "@tauri-apps/api/core";

import type { CloseBehavior, CloseChoice } from "./close-behavior";

export function listDirs() {
  return invoke<string[]>("list_dirs");
}

export function addDirs(paths: string[]) {
  return invoke<string[]>("add_dir", { paths });
}

export function removeDir(path: string) {
  return invoke<string[]>("remove_dir", { path });
}

export function pauseWatcher() {
  return invoke("pause_watcher");
}

export function resumeWatcher() {
  return invoke("resume_watcher");
}

export function reprocessAll() {
  return invoke("reprocess_all");
}

export function getCloseBehavior() {
  return invoke<CloseBehavior>("get_close_behavior");
}

export function setCloseBehavior(behavior: CloseChoice) {
  return invoke("set_close_behavior", { behavior });
}

export function minimizeToTray() {
  return invoke("minimize_to_tray");
}

export function requestAppQuit() {
  return invoke("request_app_quit");
}

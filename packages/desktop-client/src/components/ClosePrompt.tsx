import { Activity, type ReactNode } from "react";

import type { CloseChoice } from "../lib/close-behavior.ts";

type ClosePromptProps = {
  isOpen: boolean;
  rememberChoice: boolean;
  isApplying: boolean;
  onRememberChoiceChange: (checked: boolean) => void;
  onCancel: () => void;
  onChoose: (choice: CloseChoice) => void;
};

export function ClosePrompt({
  isOpen,
  rememberChoice,
  isApplying,
  onRememberChoiceChange,
  onCancel,
  onChoose,
}: ClosePromptProps): ReactNode {
  return (
    <Activity mode={isOpen ? "visible" : "hidden"} name="close-prompt">
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 p-4">
        <div className="w-full max-w-md rounded-lg border border-zinc-700 bg-zinc-900 p-4 shadow-xl">
          <h2 className="text-base font-semibold text-zinc-100">Close ROK Battles</h2>
          <p className="mt-2 text-sm text-zinc-300">
            Do you want to minimize to tray or quit the app?
          </p>
          <label className="mt-4 flex items-center gap-2 text-sm text-zinc-300">
            <input
              type="checkbox"
              checked={rememberChoice}
              onChange={(event) => onRememberChoiceChange(event.target.checked)}
              className="size-4 rounded border-zinc-600 bg-zinc-800"
            />
            Remember this option?
          </label>
          <div className="mt-5 flex items-center justify-end gap-2">
            <button
              type="button"
              onClick={onCancel}
              disabled={isApplying}
              className="rounded-md border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-sm text-zinc-200 hover:bg-zinc-700 disabled:opacity-60"
            >
              Cancel
            </button>
            <button
              type="button"
              onClick={() => onChoose("minimize_to_tray")}
              disabled={isApplying}
              className="rounded-md border border-zinc-700 bg-zinc-800 px-3 py-1.5 text-sm text-zinc-100 hover:bg-zinc-700 disabled:opacity-60"
            >
              Minimize to tray
            </button>
            <button
              type="button"
              onClick={() => onChoose("quit")}
              disabled={isApplying}
              className="rounded-md bg-red-500 px-3 py-1.5 text-sm text-white hover:bg-red-400 disabled:opacity-60"
            >
              Quit app
            </button>
          </div>
        </div>
      </div>
    </Activity>
  );
}

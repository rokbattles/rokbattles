import type { ReactNode } from "react";

import { AppHeader } from "./components/AppHeader.tsx";
import { BannerMessage } from "./components/BannerMessage.tsx";
import { ClosePrompt } from "./components/ClosePrompt.tsx";
import { Logs } from "./components/Logs.tsx";
import { WatchedDirectories } from "./components/WatchedDirectories.tsx";
import { useAppVersion } from "./hooks/useAppVersion.ts";
import { useClosePrompt } from "./hooks/useClosePrompt.ts";
import { useLogEvents } from "./hooks/useLogEvents.ts";
import { useTransientBanner } from "./hooks/useTransientBanner.ts";
import { useWatchedDirectories } from "./hooks/useWatchedDirectories.ts";

function App(): ReactNode {
  const { banner, showTransientBanner } = useTransientBanner();
  const watchedDirectories = useWatchedDirectories(showTransientBanner);
  const appVersion = useAppVersion();
  const closePrompt = useClosePrompt();
  const watcherLogs = useLogEvents("rokbattles");

  return (
    <main className="min-h-screen bg-zinc-950 text-zinc-100">
      <div className="mx-auto max-w-3xl p-6">
        <AppHeader
          isAdding={watchedDirectories.isAdding}
          isDiscovering={watchedDirectories.isDiscovering}
          isReprocessing={watchedDirectories.isReprocessing}
          onAdd={watchedDirectories.handleAdd}
          onDiscover={watchedDirectories.handleDiscover}
          onReprocess={watchedDirectories.handleReprocess}
        />
        {banner ? <BannerMessage banner={banner} /> : null}
        <WatchedDirectories
          dirs={watchedDirectories.dirs}
          isLoading={watchedDirectories.isLoading}
          onRemove={watchedDirectories.handleRemove}
        />
        <Logs logs={watcherLogs} />
        {appVersion ? (
          <footer className="pt-4 text-center text-xs text-zinc-700">Version {appVersion}</footer>
        ) : null}
      </div>
      <ClosePrompt
        isOpen={closePrompt.isOpen}
        rememberChoice={closePrompt.rememberChoice}
        isApplying={closePrompt.isApplying}
        onRememberChoiceChange={closePrompt.setRememberChoice}
        onCancel={closePrompt.cancel}
        onChoose={closePrompt.choose}
      />
    </main>
  );
}

export default App;

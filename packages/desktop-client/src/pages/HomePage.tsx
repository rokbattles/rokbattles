import type { ReactNode } from "react";

import { Banner } from "../components/Banner.tsx";
import { Button } from "../components/Button.tsx";
import { Logs } from "../components/Logs.tsx";
import { WatchedDirectories } from "../components/WatchedDirectories.tsx";
import { useBanner } from "../hooks/useBanner.ts";
import { useWatchedDirectories } from "../hooks/useWatchedDirectories.ts";
import type { LogEntry } from "../lib/log-entry.ts";

type HomePageProps = {
  logs: LogEntry[];
};

export function HomePage({ logs }: HomePageProps): ReactNode {
  const { banner, showBanner } = useBanner();
  const watchedDirectories = useWatchedDirectories(showBanner);

  return (
    <div className="py-6">
      {banner ? <Banner banner={banner} /> : null}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="text-base/6 font-semibold text-white">Scan Directories</h2>
        <div className="flex flex-wrap items-center justify-end gap-2">
          <Button
            variant="outline"
            onClick={watchedDirectories.handleDiscover}
            disabled={
              watchedDirectories.isAdding ||
              watchedDirectories.isDiscovering ||
              watchedDirectories.isReprocessing
            }
          >
            {watchedDirectories.isDiscovering ? "Scanning..." : "Auto"}
          </Button>
          <Button
            variant="outline"
            onClick={watchedDirectories.handleAdd}
            disabled={
              watchedDirectories.isAdding ||
              watchedDirectories.isDiscovering ||
              watchedDirectories.isReprocessing
            }
          >
            {watchedDirectories.isAdding ? "Adding..." : "Add"}
          </Button>
          <Button
            variant="outline"
            onClick={watchedDirectories.handleReprocess}
            disabled={watchedDirectories.isReprocessing}
          >
            {watchedDirectories.isReprocessing ? "Reprocessing..." : "Reprocess"}
          </Button>
        </div>
      </div>
      <WatchedDirectories
        dirs={watchedDirectories.dirs}
        isLoading={watchedDirectories.isLoading}
        onRemove={watchedDirectories.handleRemove}
      />
      <Logs logs={logs} />
    </div>
  );
}

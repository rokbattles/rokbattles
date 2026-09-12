import { Download } from "lucide-react";
import { downloads, releaseUrl, releaseVersion } from "../../lib/downloads";

export function DownloadOptions({ platform }: { platform: string }) {
  const builds = downloads.find((item) => item.id === platform)?.builds;
  const options =
    platform === "steamos"
      ? downloads
          .find((item) => item.id === "linux")
          ?.builds?.filter((build) => build.label === "x64 · .deb")
      : builds;

  return (
    <div className="my-6 rounded-lg border border-white/10 p-5">
      <p className="mt-0! mb-3! text-sm text-zinc-400">ROK Battles {releaseVersion}</p>
      <div className="grid gap-3 sm:grid-cols-2">
        {options?.map(({ label, file }) => (
          <a
            key={file}
            href={`${releaseUrl}/${file}`}
            className="flex min-h-11 items-center gap-3 rounded-md bg-white/5 px-4 py-3 text-sm font-medium no-underline! hover:bg-white/10"
          >
            <Download aria-hidden="true" className="size-4 shrink-0" />
            {label}
          </a>
        ))}
      </div>
    </div>
  );
}

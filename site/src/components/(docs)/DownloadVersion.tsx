import type { ReactNode } from "react";
import pkg from "../../../package.json";

export type DownloadVersionType =
  | "darwin-aarch64"
  | "darwin-aarch64-app"
  | "darwin-x86_64"
  | "darwin-x86_64-app"
  | "linux-aarch64"
  | "linux-aarch64-appimage"
  | "linux-aarch64-deb"
  | "linux-x86_64"
  | "linux-x86_64-appimage"
  | "linux-x86_64-deb"
  | "windows-aarch64"
  | "windows-aarch64-nsis"
  | "windows-x86_64"
  | "windows-x86_64-nsis";

type DownloadVersionProps = {
  type: DownloadVersionType;
  children: ReactNode;
};

function getDownloadUrl(type: DownloadVersionType, version: string) {
  const baseUrl = `https://github.com/rokbattles/rokbattles/releases/download/${version}`;

  const artifactByType: Record<DownloadVersionType, string> = {
    "darwin-aarch64": "ROK.Battles_aarch64.app.tar.gz",
    "darwin-aarch64-app": "ROK.Battles_aarch64.app.tar.gz",
    "darwin-x86_64": "ROK.Battles_x64.app.tar.gz",
    "darwin-x86_64-app": "ROK.Battles_x64.app.tar.gz",
    "linux-aarch64": `ROK.Battles_${version}_aarch64.AppImage`,
    "linux-aarch64-appimage": `ROK.Battles_${version}_aarch64.AppImage`,
    "linux-aarch64-deb": `ROK.Battles_${version}_arm64.deb`,
    "linux-x86_64": `ROK.Battles_${version}_amd64.AppImage`,
    "linux-x86_64-appimage": `ROK.Battles_${version}_amd64.AppImage`,
    "linux-x86_64-deb": `ROK.Battles_${version}_amd64.deb`,
    "windows-aarch64": `ROK.Battles_${version}_arm64-setup.exe`,
    "windows-aarch64-nsis": `ROK.Battles_${version}_arm64-setup.exe`,
    "windows-x86_64": `ROK.Battles_${version}_x64-setup.exe`,
    "windows-x86_64-nsis": `ROK.Battles_${version}_x64-setup.exe`,
  };

  return `${baseUrl}/${artifactByType[type]}`;
}

export function DownloadVersion({ type, children }: DownloadVersionProps) {
  const version = pkg.version;
  const href = getDownloadUrl(type, version);

  return <a href={href}>{children}</a>;
}

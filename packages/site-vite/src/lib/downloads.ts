import { Gamepad2, Laptop, Monitor, Smartphone, Tablet, Terminal } from "lucide-react";

export const releaseVersion = __APP_VERSION__;
export const releaseUrl = `https://github.com/rokbattles/rokbattles/releases/download/${releaseVersion}`;
export const downloads = [
  {
    id: "windows",
    name: "Windows",
    icon: Monitor,
    builds: [
      { label: "x64", file: `ROK.Battles_${releaseVersion}_x64-setup.exe` },
      { label: "ARM64", file: `ROK.Battles_${releaseVersion}_arm64-setup.exe` },
    ],
  },
  {
    id: "macos",
    name: "macOS",
    icon: Laptop,
    builds: [
      { label: "Apple Silicon", file: `ROK.Battles_${releaseVersion}_aarch64.dmg` },
      { label: "Intel", file: `ROK.Battles_${releaseVersion}_x64.dmg` },
    ],
  },
  {
    id: "linux",
    name: "Linux",
    icon: Terminal,
    builds: [
      { label: "x64 · AppImage", file: `ROK.Battles_${releaseVersion}_amd64.AppImage` },
      { label: "x64 · .deb", file: `ROK.Battles_${releaseVersion}_amd64.deb` },
      { label: "ARM64 · AppImage", file: `ROK.Battles_${releaseVersion}_aarch64.AppImage` },
      { label: "ARM64 · .deb", file: `ROK.Battles_${releaseVersion}_arm64.deb` },
    ],
  },
  {
    id: "ios",
    name: "iOS",
    icon: Tablet,
  },
  {
    id: "android",
    name: "Android",
    icon: Smartphone,
  },
  {
    id: "steamos",
    name: "SteamOS",
    icon: Gamepad2,
  },
];

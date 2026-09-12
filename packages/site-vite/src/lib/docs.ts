import { lazy } from "react";

export const installationDocs = [
  {
    slug: "windows",
    title: "Windows",
    description: "Download and install ROK Battles for Windows.",
    Content: lazy(() => import("../content/docs/installation/windows.mdx")),
  },
  {
    slug: "macos",
    title: "macOS",
    description: "Download and install ROK Battles for Apple Silicon and Intel Macs.",
    Content: lazy(() => import("../content/docs/installation/macos.mdx")),
  },
  {
    slug: "linux",
    title: "Linux",
    description: "Install ROK Battles on Linux with an AppImage or Debian package.",
    Content: lazy(() => import("../content/docs/installation/linux.mdx")),
  },
  {
    slug: "ios",
    title: "iPhone and iPad",
    description: "Install the ROK Battles configuration profile on iOS and iPadOS.",
    Content: lazy(() => import("../content/docs/installation/ios.mdx")),
  },
  {
    slug: "android",
    title: "Android",
    description: "Install Intra for use with ROK Battles on Android.",
    Content: lazy(() => import("../content/docs/installation/android.mdx")),
  },
  {
    slug: "steamos",
    title: "SteamOS",
    description: "Install ROK Battles on SteamOS using a Debian-based container.",
    Content: lazy(() => import("../content/docs/installation/steamos.mdx")),
  },
];

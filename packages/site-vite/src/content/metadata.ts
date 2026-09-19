import { Cookie, FileText, ShieldCheck } from "lucide-react";
import { lazy } from "react";

export const installationDocs = [
  {
    slug: "windows",
    title: "Windows",
    description: "Download and install ROK Battles for Windows.",
    Content: lazy(() => import("./docs/installation/windows.mdx")),
  },
  {
    slug: "macos",
    title: "macOS",
    description: "Download and install ROK Battles for Apple Silicon and Intel Macs.",
    Content: lazy(() => import("./docs/installation/macos.mdx")),
  },
  {
    slug: "linux",
    title: "Linux",
    description: "Install ROK Battles on Linux with Flatpak, an AppImage, or a Debian package.",
    Content: lazy(() => import("./docs/installation/linux.mdx")),
  },
  {
    slug: "ios",
    title: "iPhone and iPad",
    description: "Install the ROK Battles configuration profile on iOS and iPadOS.",
    Content: lazy(() => import("./docs/installation/ios.mdx")),
  },
  {
    slug: "android",
    title: "Android",
    description: "Install Intra for use with ROK Battles on Android.",
    Content: lazy(() => import("./docs/installation/android.mdx")),
  },
  {
    slug: "steamos",
    title: "SteamOS",
    description: "Install ROK Battles on SteamOS with Flatpak in Desktop Mode.",
    Content: lazy(() => import("./docs/installation/steamos.mdx")),
  },
];

export const legalDocuments = [
  {
    id: "terms-of-service",
    title: "Terms of Service",
    description: "The rules and conditions for using ROK Battles and its services.",
    icon: FileText,
    Content: lazy(() => import("./legal/terms-of-service.md")),
  },
  {
    id: "privacy-policy",
    title: "Privacy Policy",
    description: "How ROK Battles collects, uses, and protects personal data.",
    icon: ShieldCheck,
    Content: lazy(() => import("./legal/privacy-policy.md")),
  },
  {
    id: "cookie-policy",
    title: "Cookie Policy",
    description: "How ROK Battles uses cookies and similar technologies.",
    icon: Cookie,
    Content: lazy(() => import("./legal/cookie-policy.md")),
  },
];

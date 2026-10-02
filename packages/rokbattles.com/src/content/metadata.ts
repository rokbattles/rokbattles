import { Cookie, FileText, ShieldCheck } from "lucide-react";

export const installationDocs = [
  {
    slug: "windows",
    title: "Windows",
    description: "Download and install ROK Battles for Windows.",
  },
  {
    slug: "macos",
    title: "macOS",
    description: "Download and install ROK Battles for Apple Silicon and Intel Macs.",
  },
  {
    slug: "linux",
    title: "Linux",
    description: "Install ROK Battles on Linux with Flatpak, an AppImage, or a Debian package.",
  },
  {
    slug: "ios",
    title: "iPhone and iPad",
    description: "Install the ROK Battles configuration profile on iOS and iPadOS.",
  },
  {
    slug: "android",
    title: "Android",
    description: "Install Intra for use with ROK Battles on Android.",
  },
  {
    slug: "steamos",
    title: "SteamOS",
    description: "Install ROK Battles on SteamOS with Flatpak in Desktop Mode.",
  },
];

export const legalDocuments = [
  {
    id: "terms-of-service",
    title: "Terms of Service",
    description: "The rules and conditions for using ROK Battles and its services.",
    icon: FileText,
  },
  {
    id: "privacy-policy",
    title: "Privacy Policy",
    description: "How ROK Battles collects, uses, and protects personal data.",
    icon: ShieldCheck,
  },
  {
    id: "cookie-policy",
    title: "Cookie Policy",
    description: "How ROK Battles uses cookies and similar technologies.",
    icon: Cookie,
  },
];

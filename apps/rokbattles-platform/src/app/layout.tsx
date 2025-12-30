import "./globals.css";
import type { Metadata } from "next";
import { Inter } from "next/font/google";
import { CookieConsentBanner } from "@/components/CookieConsentBanner";
import { PlatformLayout } from "@/components/PlatformLayout";
import PlatformProviders from "@/components/PlatformProviders";
import { cn } from "@/lib/cn";
import { getCurrentUser } from "@/lib/current-user";
import { CookieConsentProvider } from "@/providers/CookieConsentContext";

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-inter",
  display: "swap",
});

export const metadata: Metadata = {
  metadataBase: new URL("https://platform.rokbattles.com"),
  title: {
    default: "ROK Battles",
    template: "%s - ROK Battles",
  },
  description:
    "A community-driven platform for sharing battle reports and surfacing actionable trends in Rise of Kingdoms",
  openGraph: {
    title: {
      default: "ROK Battles",
      template: "%s - ROK Battles",
    },
  },
  twitter: {
    title: {
      default: "ROK Battles",
      template: "%s - ROK Battles",
    },
  },
};

export default async function Layout({ children }: LayoutProps<"/">) {
  const user = await getCurrentUser();
  const initialGovernors = user?.claimedGovernors ?? [];
  const initialActiveGovernorId = initialGovernors[0]?.governorId;

  return (
    <html
      lang="en"
      className={cn(
        inter.variable,
        "text-zinc-950 antialiased lg:bg-zinc-100 dark:bg-zinc-900 dark:text-white dark:lg:bg-zinc-950"
      )}
      suppressHydrationWarning
    >
      <head>
        <link rel="preconnect" href="https://plat-fau-global.lilithgame.com" crossOrigin="" />
        <link rel="dns-prefetch" href="https://imimg.lilithcdn.com" />
        <link rel="dns-prefetch" href="https://imv2-gl.lilithgame.com" />
        <link rel="dns-prefetch" href="https://static-gl.lilithgame.com" />
      </head>
      <body>
        <CookieConsentProvider>
          <PlatformProviders
            initialGovernors={initialGovernors}
            initialActiveGovernorId={initialActiveGovernorId}
          >
            <PlatformLayout initialUser={user}>{children}</PlatformLayout>
          </PlatformProviders>
          <CookieConsentBanner />
        </CookieConsentProvider>
      </body>
    </html>
  );
}

import "./globals.css";
import type { Metadata } from "next";
import { Inter } from "next/font/google";
import { NextIntlClientProvider } from "next-intl";
import { getLocale, getMessages, getTranslations } from "next-intl/server";
import { CookieConsentBanner } from "@/components/cookie-consent-banner";
import { PlatformLayout } from "@/components/platform-layout";
import PlatformProviders from "@/components/platform-providers";
import { cn } from "@/lib/cn";
import { getCurrentUser } from "@/lib/current-user";
import { CookieConsentProvider } from "@/providers/cookie-consent-context";

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-inter",
  display: "swap",
});

const metadataBase = new URL("https://platform.rokbattles.com");

export async function generateMetadata(): Promise<Metadata> {
  const t = await getTranslations("app");
  const title = t("title");
  const template = t("titleTemplate");
  const description = t("description");

  return {
    metadataBase,
    title: {
      default: title,
      template,
    },
    description,
    openGraph: {
      title: {
        default: title,
        template,
      },
    },
    twitter: {
      title: {
        default: title,
        template,
      },
    },
  };
}

export default async function Layout({ children }: LayoutProps<"/">) {
  const user = await getCurrentUser();
  const initialGovernors = user?.claimedGovernors ?? [];
  const initialActiveGovernorId = initialGovernors[0]?.governorId;

  // next-intl
  const locale = await getLocale();
  const messages = await getMessages();

  return (
    <html
      lang={locale}
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
        <NextIntlClientProvider messages={messages}>
          <CookieConsentProvider>
            <PlatformProviders
              initialGovernors={initialGovernors}
              initialActiveGovernorId={initialActiveGovernorId}
            >
              <PlatformLayout initialUser={user}>{children}</PlatformLayout>
            </PlatformProviders>
            <CookieConsentBanner />
          </CookieConsentProvider>
        </NextIntlClientProvider>
      </body>
    </html>
  );
}

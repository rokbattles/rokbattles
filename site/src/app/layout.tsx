import "./globals.css";
import type { Metadata } from "next";
import { Inter } from "next/font/google";
import { NextIntlClientProvider } from "next-intl";
import { getLocale, getMessages } from "next-intl/server";
import { CookieConsentBanner } from "@/components/cookie-consent-banner";
import { cn } from "@/lib/cn";
import { CookieConsentProvider } from "@/providers/cookie-consent-context";

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-inter",
  display: "swap",
});

export const metadata: Metadata = {
  metadataBase: new URL("https://rokbattles.com"),
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
        <link rel="dns-prefetch" href="https://cdn.rokbattles.com" />
      </head>
      <body>
        <NextIntlClientProvider messages={messages}>
          <CookieConsentProvider>
            {children}
            <CookieConsentBanner />
          </CookieConsentProvider>
        </NextIntlClientProvider>
      </body>
    </html>
  );
}

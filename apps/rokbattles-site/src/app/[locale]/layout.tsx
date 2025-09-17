import { Inter } from "next/font/google";
import { cn } from "@/lib/cn";
import "../globals.css";
import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { hasLocale, NextIntlClientProvider } from "next-intl";
import { setRequestLocale } from "next-intl/server";
import { routing } from "@/i18n/routing";

const inter = Inter({
  subsets: ["latin"],
  display: "swap",
  variable: "--font-inter",
});

export const metadata: Metadata = {
  metadataBase: new URL("https://rokbattles.com"),
  title: {
    default: "ROK Battles",
    template: "%s - ROK Battles",
  },
  description:
    "A community-driven platform for sharing battle reports and surfacing actionable trends in Rise of Kingdoms",
};

export default async function LocaleLayout({ children, params }: LayoutProps<"/[locale]">) {
  const { locale } = await params;
  if (!hasLocale(routing.locales, locale)) {
    notFound();
  }

  setRequestLocale(locale);

  return (
    <html lang={locale} className={cn(inter.variable, "antialiased")}>
      <body>
        <NextIntlClientProvider>
          <div className="isolate">{children}</div>
        </NextIntlClientProvider>
      </body>
    </html>
  );
}

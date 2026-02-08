import { getLocale } from "next-intl/server";
import "./globals.css";
import { Inter } from "next/font/google";
import { NextIntlClientProvider } from "next-intl";
import AppLayout from "@/components/app-layout";
import { cn } from "@/lib/cn";

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-inter",
  display: "swap",
});

export default async function RootLayout({ children }: LayoutProps<"/">) {
  const locale = await getLocale();

  return (
    <html
      className={cn(
        inter.variable,
        "text-zinc-950 antialiased lg:bg-zinc-100 dark:bg-zinc-900 dark:text-white dark:lg:bg-zinc-950"
      )}
      lang={locale}
    >
      <body>
        <NextIntlClientProvider>
          <AppLayout>{children}</AppLayout>
        </NextIntlClientProvider>
      </body>
    </html>
  );
}

import { getLocale } from "next-intl/server";
import type { ReactNode } from "react";
import "./globals.css";
import { NextIntlClientProvider } from "next-intl";
import AppLayout from "@/components/app-layout";

export default async function RootLayout({
  children,
}: {
  children: ReactNode;
}) {
  const locale = await getLocale();

  return (
    <html
      className="text-zinc-950 antialiased lg:bg-zinc-100 dark:bg-zinc-900 dark:text-white dark:lg:bg-zinc-950"
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

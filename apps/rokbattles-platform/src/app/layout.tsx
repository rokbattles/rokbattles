import { getLocale } from "next-intl/server";
import type { ReactNode } from "react";
import "./globals.css";
import { NextIntlClientProvider } from "next-intl";

export default async function RootLayout({
  children,
}: {
  children: ReactNode;
}) {
  const locale = await getLocale();

  return (
    <html lang={locale}>
      <body>
        <NextIntlClientProvider>{children}</NextIntlClientProvider>
      </body>
    </html>
  );
}

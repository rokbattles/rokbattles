import type { Metadata } from "next";
import type { ReactNode } from "react";
import "../(legacy)/globals.css";

export const metadata: Metadata = {
  robots: { index: false, follow: false },
  referrer: "no-referrer",
};

export default function OverlayLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" className="overflow-hidden bg-transparent">
      <body className="overflow-hidden bg-transparent [&_nextjs-portal]:hidden">{children}</body>
    </html>
  );
}

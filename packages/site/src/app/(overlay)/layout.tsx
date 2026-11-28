import type { Metadata } from "next";
import type { ReactNode } from "react";
import "../(legacy)/globals.css";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

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

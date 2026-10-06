import { cn } from "cn";
import { Inter } from "next/font/google";
import "./globals.css";
import type { Metadata } from "next";
import { MarketingFooter } from "@/components/(marketing)/footer";
import { MarketingHeader } from "@/components/(marketing)/header";
import { frame } from "@/components/(marketing)/ui/layout";

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
    description:
      "A community-driven platform for sharing battle reports and surfacing actionable trends in Rise of Kingdoms",
    siteName: "ROK Battles",
    type: "website",
  },
  twitter: {
    title: {
      default: "ROK Battles",
      template: "%s - ROK Battles",
    },
    description:
      "A community-driven platform for sharing battle reports and surfacing actionable trends in Rise of Kingdoms",
    card: "summary",
  },
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en" className={cn(inter.variable, "antialiased")} data-scroll-behavior="smooth">
      <body>
        <div className="flex min-h-dvh flex-col bg-zinc-950 text-white selection:bg-orange-400 selection:text-zinc-950">
          <a
            href="#main"
            className="sr-only focus:fixed focus:top-4 focus:left-4 focus:z-50 focus:m-0 focus:h-auto focus:w-auto focus:overflow-visible focus:rounded-sm focus:bg-zinc-900 focus:px-4 focus:py-3 focus:[clip:auto] focus-visible:outline-2 focus-visible:outline-offset-4 focus-visible:outline-orange-400"
          >
            Skip to content
          </a>
          <MarketingHeader />
          <div className={cn(frame, "flex flex-1 flex-col")}>
            <main id="main" tabIndex={-1} className="flex flex-1 flex-col">
              {children}
            </main>
            <MarketingFooter />
          </div>
        </div>
      </body>
    </html>
  );
}

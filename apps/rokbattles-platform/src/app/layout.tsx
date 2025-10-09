import "./globals.css";
import { Inter } from "next/font/google";
import { PlatformLayout } from "@/components/PlatformLayout";
import PlatformProviders from "@/components/PlatformProviders";
import { cn } from "@/lib/cn";

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-inter",
  display: "swap",
});

export default function Layout({ children }: LayoutProps<"/">) {
  return (
    <html
      lang="en"
      className={cn(
        inter.variable,
        "text-zinc-950 antialiased lg:bg-zinc-100 dark:bg-zinc-900 dark:text-white dark:lg:bg-zinc-950"
      )}
    >
      <head>
        <link rel="preconnect" href="https://plat-fau-global.lilithgame.com" crossOrigin="" />
        <link rel="dns-prefetch" href="https://imimg.lilithcdn.com" />
        <link rel="dns-prefetch" href="https://imv2-gl.lilithgame.com" />
        <link rel="dns-prefetch" href="https://static-gl.lilithgame.com" />
      </head>
      <body>
        <PlatformProviders>
          <PlatformLayout>{children}</PlatformLayout>
        </PlatformProviders>
      </body>
    </html>
  );
}

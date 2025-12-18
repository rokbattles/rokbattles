import { Inter } from "next/font/google";
import { cn } from "@/lib/cn";
import "./globals.css";
import type { Metadata } from "next";

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

export default async function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en" className={cn(inter.variable, "antialiased")}>
      <body>{children}</body>
    </html>
  );
}

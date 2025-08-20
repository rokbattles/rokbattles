import type { ReactNode } from "react";
import "./globals.css";
import { Inter } from "next/font/google";
import { cn } from "../lib/cn";

const inter = Inter({
  subsets: ["latin"],
  display: "swap",
  variable: "--font-inter",
});

export default function Layout({ children }: { children: Readonly<ReactNode> }) {
  return (
    <html
      lang="en"
      className={cn(inter.variable, "antialiased text-white bg-zinc-900 lg:bg-zinc-950")}
    >
      <body>{children}</body>
    </html>
  );
}

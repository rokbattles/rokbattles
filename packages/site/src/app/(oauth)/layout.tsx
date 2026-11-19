import { Inter } from "next/font/google";
import Image from "next/image";
import type { ReactNode } from "react";
import "../(legacy)/globals.css";

const inter = Inter({ subsets: ["latin"], variable: "--font-inter", display: "swap" });

export default function Layout({ children }: { children: ReactNode }) {
  return (
    <html lang="en" className={`${inter.variable} bg-zinc-950 font-sans text-white antialiased`}>
      <body>
        <main className="grid min-h-dvh place-items-center bg-[radial-gradient(ellipse_at_50%_0%,#3b2318_0%,#1a1515_38%,#101012_74%)] px-4 py-8 sm:px-6 sm:py-14">
          <div className="w-full max-w-lg overflow-hidden rounded-2xl border border-white/10 bg-[#19181b] shadow-2xl shadow-black/50">
            <div
              aria-hidden="true"
              className="h-0.5 bg-linear-to-r from-orange-500 via-orange-400/55 to-transparent"
            />
            <div className="px-6 pt-6 sm:px-8 sm:pt-8">
              <Image
                src="https://cdn.rokbattles.com/assets/logo.svg"
                alt="ROK Battles"
                width={2104}
                height={556}
                className="h-auto w-36 sm:w-40"
              />
            </div>
            {children}
          </div>
        </main>
      </body>
    </html>
  );
}

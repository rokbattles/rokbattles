import { RootProvider } from "fumadocs-ui/provider/next";
import { DiscordEmbed } from "@/components/discord-embed";
import "./globals.css";

export default function Layout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <DiscordEmbed />
      </head>
      <body className="flex flex-col min-h-screen">
        <RootProvider>{children}</RootProvider>
      </body>
    </html>
  );
}

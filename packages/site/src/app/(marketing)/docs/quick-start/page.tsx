import { MarkdownDocument } from "@/components/(marketing)/markdown-document";
import QuickStart from "@/content/docs/quick-start.mdx";

export const metadata = {
  title: "Quick Start",
  description: "Connect your mailcache folder and start uploading reports with ROK Battles.",
};

export default function QuickStartPage() {
  return <MarkdownDocument Content={QuickStart} title="Quick Start" />;
}

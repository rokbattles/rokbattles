import { MarkdownDocument } from "@/components/markdown-document";
import GettingStarted from "@/content/docs/getting-started.md";

export const metadata = {
  title: "Docs",
  description:
    "Download and install ROK Battles on desktop and mobile. Choose a platform to get started.",
};

export default function DocsPage() {
  return <MarkdownDocument Content={GettingStarted} title="Get started with ROK Battles" />;
}

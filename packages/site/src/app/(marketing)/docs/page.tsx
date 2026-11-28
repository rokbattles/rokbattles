import { MarkdownDocument } from "@/components/(marketing)/markdown-document";
import GettingStarted from "@/content/docs/getting-started.md";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export const metadata = {
  title: "Docs",
  description:
    "Download and install ROK Battles on desktop and mobile. Choose a platform to get started.",
};

export default function DocsPage() {
  return <MarkdownDocument Content={GettingStarted} title="Get started with ROK Battles" />;
}

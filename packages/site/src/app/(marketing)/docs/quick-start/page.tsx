import { MarkdownDocument } from "@/components/(marketing)/markdown-document";
import QuickStart from "@/content/docs/quick-start.mdx";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export const metadata = {
  title: "Quick Start",
  description: "Connect your mailcache folder and start uploading reports with ROK Battles.",
};

export default function QuickStartPage() {
  return <MarkdownDocument Content={QuickStart} title="Quick Start" />;
}

import { MarkdownDocument } from "@/components/(marketing)/markdown-document";
import MigratingReportManagers from "@/content/docs/migrating-report-managers.mdx";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export const metadata = {
  title: "Migrating Report Managers",
  description: "Import reports saved by another report manager into ROK Battles.",
};

export default function MigratingReportManagersPage() {
  return <MarkdownDocument Content={MigratingReportManagers} title="Migrating Report Managers" />;
}

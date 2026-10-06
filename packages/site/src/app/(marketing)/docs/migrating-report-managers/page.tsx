import { MarkdownDocument } from "@/components/(marketing)/markdown-document";
import MigratingReportManagers from "@/content/docs/migrating-report-managers.mdx";

export const metadata = {
  title: "Migrating Report Managers",
  description: "Import reports saved by another report manager into ROK Battles.",
};

export default function MigratingReportManagersPage() {
  return <MarkdownDocument Content={MigratingReportManagers} title="Migrating Report Managers" />;
}

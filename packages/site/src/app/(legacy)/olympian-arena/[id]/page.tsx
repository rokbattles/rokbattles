import DuelReportView from "@/components/olympian-arena/duel-report-view";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default async function Page({ params }: PageProps<"/olympian-arena/[id]">) {
  const { id } = await params;

  return <DuelReportView duelId={id ?? ""} />;
}

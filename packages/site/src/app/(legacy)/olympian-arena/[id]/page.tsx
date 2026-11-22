import DuelReportView from "@/components/olympian-arena/duel-report-view";

export default async function Page({ params }: PageProps<"/olympian-arena/[id]">) {
  const { id } = await params;

  return <DuelReportView duelId={id ?? ""} />;
}

import { ReportView } from "@/components/report/ReportView";

export default async function Page({ params }: PageProps<"/report/[hash]">) {
  const { hash } = await params;

  return (
    <div className="space-y-8">
      <ReportView hash={hash ?? ""} />
    </div>
  );
}

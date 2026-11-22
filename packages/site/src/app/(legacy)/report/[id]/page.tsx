import type { Metadata } from "next";

import { ReportView } from "@/components/report/report-view";

export async function generateMetadata(): Promise<Metadata> {
  const title = "Battle Report";

  return {
    title,
  };
}

export default async function Page({ params }: PageProps<"/report/[id]">) {
  const { id } = await params;
  return <ReportView id={id ?? ""} />;
}

import { Heading } from "@/components/ui/Heading";

export default async function ReportPage({ params }: PageProps<"/[locale]/app/report/[hash]">) {
  const { hash } = await params;

  return <Heading>Battle Report - {hash}</Heading>;
}

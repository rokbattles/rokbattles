import type { Metadata } from "next";

import { ReportView } from "@/components/report/report-view";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

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

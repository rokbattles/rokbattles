import OlympianArenaTable from "@/components/olympian-arena/olympian-arena-table";
import { Heading, Subheading } from "@/components/ui/heading";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default async function Page() {
  return (
    <>
      <Heading>Olympian Arena Reports</Heading>
      <div className="mt-8">
        <Subheading>Live feed (UTC)</Subheading>
      </div>
      <OlympianArenaTable />
    </>
  );
}

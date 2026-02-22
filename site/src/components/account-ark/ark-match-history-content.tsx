import { ArkMatchHistoryEmptyState } from "@/components/account-ark/ark-match-history-empty-state";
import { ArkMatchHistoryErrorState } from "@/components/account-ark/ark-match-history-error-state";
import { ArkMatchHistoryTable } from "@/components/account-ark/ark-match-history-table";
import { getGovernorArkMatchHistory } from "@/data/ark/query";

type ArkMatchHistoryContentProps = {
  governorId: number;
  limit?: number;
};

export async function ArkMatchHistoryContent({ governorId, limit }: ArkMatchHistoryContentProps) {
  try {
    const data = await getGovernorArkMatchHistory({ governorId, limit });

    if (data.rows.length === 0) {
      return <ArkMatchHistoryEmptyState />;
    }

    return (
      <section className="mt-8 space-y-4">
        <ArkMatchHistoryTable rows={data.rows} />
      </section>
    );
  } catch (error) {
    console.error("Failed to load ark match history", error);
    return <ArkMatchHistoryErrorState />;
  }
}

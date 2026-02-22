import { ArkMatchDetailErrorState } from "@/components/account-ark/ark-match-detail-error-state";
import { ArkMatchDetailIndividualResultsSection } from "@/components/account-ark/ark-match-detail-individual-results-section";
import { ArkMatchDetailNotFoundState } from "@/components/account-ark/ark-match-detail-not-found-state";
import { ArkMatchDetailOverviewSection } from "@/components/account-ark/ark-match-detail-overview-section";
import { ArkMatchDetailPairingsSection } from "@/components/account-ark/ark-match-detail-pairings-section";
import { getGovernorArkMatchDetail } from "@/data/ark/query";

type ArkMatchDetailContentProps = {
  governorId: number;
  matchId: string;
};

export async function ArkMatchDetailContent({ governorId, matchId }: ArkMatchDetailContentProps) {
  try {
    const detail = await getGovernorArkMatchDetail({ governorId, matchId });
    if (!detail) {
      return <ArkMatchDetailNotFoundState />;
    }

    return (
      <section className="mt-8 space-y-6">
        <ArkMatchDetailOverviewSection overview={detail.overview} />
        <ArkMatchDetailIndividualResultsSection individualResults={detail.individualResults} />
        <ArkMatchDetailPairingsSection pairings={detail.pairings} />
      </section>
    );
  } catch (error) {
    console.error("Failed to load ark match detail", error);
    return <ArkMatchDetailErrorState />;
  }
}

import { CommunityCta } from "@/components/(marketing)/community-cta";
import { CommunityMetrics } from "@/components/(marketing)/community-metrics";
import { CommunityTools } from "@/components/(marketing)/community-tools";
import { CreatorTestimonials } from "@/components/(marketing)/creator-testimonials";
import { Downloads } from "@/components/(marketing)/downloads";
import { Drastc } from "@/components/(marketing)/drastc";
import { Hero } from "@/components/(marketing)/hero";
import { ProjectSupport } from "@/components/(marketing)/project-support";
import { ReportOverview } from "@/components/(marketing)/report-overview";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default function HomePage() {
  return (
    <>
      <Hero />
      <CommunityMetrics />
      <ReportOverview />
      <CreatorTestimonials />
      <CommunityTools />
      <Drastc />
      <Downloads />
      <ProjectSupport />
      <CommunityCta />
    </>
  );
}

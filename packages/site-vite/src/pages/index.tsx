import { CommunityCta } from "../components/marketing/community-cta";
import { CommunityMetrics } from "../components/marketing/community-metrics";
import { CommunityTools } from "../components/marketing/community-tools";
import { CreatorTestimonials } from "../components/marketing/creator-testimonials";
import { Downloads } from "../components/marketing/downloads";
import { Drastc } from "../components/marketing/drastc";
import { Hero } from "../components/marketing/hero";
import { ProjectSupport } from "../components/marketing/project-support";
import { ReportOverview } from "../components/marketing/report-overview";
import Metadata from "../components/metadata";

export default function IndexRoute() {
  return (
    <>
      <Metadata />

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

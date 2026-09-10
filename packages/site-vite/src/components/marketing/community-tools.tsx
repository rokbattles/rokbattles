import { FlaskConical, Gem, Map as MapIcon } from "lucide-react";
import { Heading } from "../ui/marketing/heading";
import { card, GridMarkers, sectionHeading } from "../ui/marketing/layout";
import { Subheading } from "../ui/marketing/subheading";
import { Text } from "../ui/marketing/text";

const communityTools = [
  {
    icon: FlaskConical,
    title: "Combat Lab",
    description:
      "Explore commander pairing performance using battle data across community-collected data.",
  },
  {
    icon: MapIcon,
    title: "Territory Planner",
    description:
      "Design and share alliance territory plans with accurate structures, boundaries, and resource costs.",
  },
  {
    icon: Gem,
    title: "Loot Explorer",
    description:
      "Explore potential loot, drop rates, quantity ranges, and sample counts across community-collected data.",
  },
];

export function CommunityTools() {
  return (
    <section id="community-tools" aria-label="Community tools" className="relative scroll-mt-8">
      <GridMarkers />
      <div className={sectionHeading}>
        <Heading id="overview-heading">More than battle reports.</Heading>
      </div>
      <div className="grid gap-px border-y border-white/10 bg-white/10 lg:grid-cols-3">
        {communityTools.map(({ icon: Icon, title, description }) => (
          <article key={title} className={card}>
            <div className="mb-8 flex items-center justify-between">
              <Icon className="size-6 text-orange-400" strokeWidth={1.5} aria-hidden="true" />
            </div>
            <Subheading>{title}</Subheading>
            <Text className="mt-4">{description}</Text>
          </article>
        ))}
      </div>
    </section>
  );
}

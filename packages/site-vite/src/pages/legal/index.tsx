import { cn } from "cn";
import { ArrowRight } from "lucide-react";
import Metadata from "../../components/metadata";
import { Button } from "../../components/ui/marketing/button";
import { Heading } from "../../components/ui/marketing/heading";
import { card, GridMarkers, gutter } from "../../components/ui/marketing/layout";
import { Link } from "../../components/ui/marketing/link";
import { Subheading } from "../../components/ui/marketing/subheading";
import { Text } from "../../components/ui/marketing/text";
import { legalDocuments } from "../../lib/legal-documents";

export default function LegalRoute() {
  return (
    <>
      <Metadata
        title="Legal"
        description="Policies, privacy, and cookie preferences for ROK Battles."
      />

      <div className={cn(gutter, "border-b border-white/10 py-12 sm:py-16")}>
        <Heading level={1}>Legal</Heading>
        <Text className="mt-6 max-w-md">
          Policies, privacy, and cookie preferences for ROK Battles.
        </Text>
      </div>
      <div className="relative grid gap-px border-b border-white/10 bg-white/10 lg:grid-cols-[3fr_2fr]">
        <GridMarkers />
        <section aria-label="Legal documents" className="grid gap-px">
          {legalDocuments.map(({ id, title, description, icon: Icon }) => (
            <article key={id} id={id} className={cn(card, "scroll-mt-8")}>
              <div className="flex items-center gap-3">
                <Icon
                  aria-hidden="true"
                  className="size-5 shrink-0 text-orange-400"
                  strokeWidth={1.5}
                />
                <Subheading level={2}>{title}</Subheading>
              </div>
              <Text className="mt-3">{description}</Text>
              <Link href={`/legal/${id}`} className="mt-4" aria-label={`Read ${title}`}>
                Read document <ArrowRight aria-hidden="true" className="size-4" />
              </Link>
            </article>
          ))}
        </section>
        <section
          id="cookie-settings"
          aria-labelledby="cookie-settings-heading"
          className={cn(card, "scroll-mt-8")}
        >
          <Subheading level={2} id="cookie-settings-heading">
            Cookie settings
          </Subheading>
          <Text className="mt-3 max-w-sm">Review and manage your cookie preferences.</Text>
          <Button disabled className="mt-6">
            Manage cookies
          </Button>
        </section>
      </div>
    </>
  );
}

import { cn } from "cn";
import { ArrowRight } from "lucide-react";
import { Heading } from "../../../components/(marketing)/ui/heading";
import { card, GridMarkers, gutter } from "../../../components/(marketing)/ui/layout";
import { Link } from "../../../components/(marketing)/ui/link";
import { Subheading } from "../../../components/(marketing)/ui/subheading";
import { Text } from "../../../components/(marketing)/ui/text";
import { legalDocuments } from "../../../content/metadata";

export const metadata = {
  title: "Legal",
  description: "Terms and privacy policies for ROK Battles.",
};

export default function LegalPage() {
  return (
    <>
      <div className={cn(gutter, "border-b border-white/10 py-12 sm:py-16")}>
        <Heading level={1}>Legal</Heading>
        <Text className="mt-6 max-w-md">Terms and privacy policies for ROK Battles.</Text>
      </div>
      <div className="relative grid gap-px border-b border-white/10 bg-white/10 ">
        <GridMarkers />
        <section aria-label="Legal documents" className="grid gap-px lg:grid-cols-3">
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
      </div>
    </>
  );
}

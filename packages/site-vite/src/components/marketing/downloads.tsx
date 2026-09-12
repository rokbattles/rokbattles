import { cn } from "cn";
import { BookOpen, ChevronDown } from "lucide-react";
import { downloads, releaseUrl } from "../../lib/downloads";
import { Button } from "../ui/marketing/button";
import { Dropdown, DropdownButton, DropdownItem, DropdownMenu } from "../ui/marketing/dropdown";
import { Heading } from "../ui/marketing/heading";
import { card, GridMarkers, sectionHeading } from "../ui/marketing/layout";
import { Subheading } from "../ui/marketing/subheading";

export function Downloads() {
  return (
    <section id="downloads" aria-label="Downloads" className="relative border-b border-white/10">
      <GridMarkers />
      <div className={sectionHeading}>
        <Heading id="downloads-heading">ROK Battles, wherever you play.</Heading>
      </div>
      <div className="grid gap-px border-t border-white/10 bg-white/10 md:grid-cols-2 lg:grid-cols-3">
        {downloads.map(({ id, name, icon: Icon, builds }) => (
          <article key={name} className={cn(card, "flex min-w-0 flex-col")}>
            <Icon className="mb-8 size-6 text-orange-400" strokeWidth={1.5} aria-hidden="true" />
            <Subheading>{name}</Subheading>
            <div className="mt-8 flex items-stretch gap-2">
              {builds ? (
                <Dropdown>
                  <DropdownButton render={<Button className="w-full" />}>
                    Download for {name} <ChevronDown aria-hidden="true" />
                  </DropdownButton>
                  <DropdownMenu align="end" aria-label={`${name} builds`}>
                    {builds.map(({ label, file }) => (
                      <DropdownItem key={file} href={`${releaseUrl}/${file}`}>
                        {label}
                      </DropdownItem>
                    ))}
                  </DropdownMenu>
                </Dropdown>
              ) : (
                <Button href={`/docs/installation/${id}`} className="w-full">
                  {name} install guide <BookOpen aria-hidden="true" />
                </Button>
              )}
            </div>
          </article>
        ))}
      </div>
    </section>
  );
}

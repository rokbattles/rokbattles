import clsx from "clsx";
import {
  BookOpen,
  ChevronDown,
  Gamepad2,
  Laptop,
  Monitor,
  Smartphone,
  Tablet,
  Terminal,
} from "lucide-react";
import { Button } from "../ui/marketing/button";
import { Dropdown, DropdownButton, DropdownItem, DropdownMenu } from "../ui/marketing/dropdown";
import { Heading } from "../ui/marketing/heading";
import { card, GridMarkers, sectionHeading } from "../ui/marketing/layout";
import { Subheading } from "../ui/marketing/subheading";

const releaseVersion = "1.6.1";
const releaseUrl = `https://github.com/rokbattles/rokbattles/releases/download/${releaseVersion}`;
const downloads = [
  {
    name: "Windows",
    icon: Monitor,
    builds: [
      { label: "x64", file: `ROK.Battles_${releaseVersion}_x64-setup.exe` },
      { label: "ARM64", file: `ROK.Battles_${releaseVersion}_arm64-setup.exe` },
    ],
  },
  {
    name: "macOS",
    icon: Laptop,
    builds: [
      { label: "Apple Silicon", file: `ROK.Battles_${releaseVersion}_aarch64.dmg` },
      { label: "Intel", file: `ROK.Battles_${releaseVersion}_x64.dmg` },
    ],
  },
  {
    name: "Linux",
    icon: Terminal,
    builds: [
      { label: "x64 · AppImage", file: `ROK.Battles_${releaseVersion}_amd64.AppImage` },
      { label: "x64 · .deb", file: `ROK.Battles_${releaseVersion}_amd64.deb` },
      { label: "ARM64 · AppImage", file: `ROK.Battles_${releaseVersion}_aarch64.AppImage` },
      { label: "ARM64 · .deb", file: `ROK.Battles_${releaseVersion}_arm64.deb` },
    ],
  },
  {
    name: "iOS",
    icon: Tablet,
  },
  {
    name: "Android",
    icon: Smartphone,
  },
  {
    name: "SteamOS",
    icon: Gamepad2,
  },
];

export function Downloads() {
  return (
    <section id="downloads" aria-label="Downloads" className="relative border-b border-white/10">
      <GridMarkers />
      <div className={sectionHeading}>
        <Heading id="downloads-heading">ROK Battles, wherever you play.</Heading>
      </div>
      <div className="grid gap-px border-t border-white/10 bg-white/10 md:grid-cols-2 lg:grid-cols-3">
        {downloads.map(({ name, icon: Icon, builds }) => (
          <article key={name} className={clsx(card, "flex min-w-0 flex-col")}>
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
                <Button type="button" disabled className="w-full">
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

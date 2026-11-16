import { ChevronRight } from "lucide-react";
import { Link } from "@/components/ui/link";

type SettingsLinkProps = {
  href: string;
  label: string;
  value: string;
};

export function SettingsLink({ href, label, value }: SettingsLinkProps) {
  return (
    <Link
      href={href}
      className="group flex min-h-14 items-center justify-between gap-4 rounded-lg px-3 py-3 text-sm hover:bg-white/5 focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white"
    >
      <span>{label}</span>
      <span className="inline-flex shrink-0 items-center gap-2 text-zinc-400 group-hover:text-white">
        {value}
        <ChevronRight aria-hidden="true" className="size-4" />
      </span>
    </Link>
  );
}

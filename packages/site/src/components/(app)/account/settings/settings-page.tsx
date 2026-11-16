import { ChevronLeft } from "lucide-react";
import type { ReactNode } from "react";
import { Link } from "@/components/ui/link";

type SettingsPageProps = {
  backHref?: string;
  backLabel?: string;
  children: ReactNode;
};

export function SettingsPage({ backHref, backLabel = "Account", children }: SettingsPageProps) {
  return (
    <div className="space-y-8 pt-4">
      {backHref ? (
        <Link
          href={backHref}
          className="inline-flex items-center gap-1 text-sm text-zinc-400 hover:text-white"
        >
          <ChevronLeft aria-hidden="true" className="size-4" />
          {backLabel}
        </Link>
      ) : null}
      {children}
    </div>
  );
}

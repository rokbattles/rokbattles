import { cn } from "cn";
import { gutter } from "../ui/marketing/layout";
import { Link } from "../ui/marketing/link";

export function MarketingFooter() {
  return (
    <footer className={cn(gutter, "flex flex-wrap items-center justify-between gap-6 py-8")}>
      <Link href="/" className="shrink-0">
        <img
          src="/assets/marketing/logo.svg"
          alt="ROK Battles"
          width={2104}
          height={556.24}
          className="h-auto w-36"
        />
      </Link>
      <div className="flex flex-wrap items-center gap-x-6 gap-y-1">
        {[
          { label: "Legal", href: "/legal" },
          { label: "Cookie settings", href: "#" },
        ].map(({ label, href }) => (
          <Link key={label} href={href}>
            {label}
          </Link>
        ))}
      </div>
    </footer>
  );
}

import { cn } from "cn";
import { use } from "react";
import { CookieConsentContext } from "../../providers/cookie-consent-context";
import { gutter } from "../ui/marketing/layout";
import { Link } from "../ui/marketing/link";

export function MarketingFooter() {
  const { open } = use(CookieConsentContext);

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
        <Link href="/legal">Legal</Link>
        <Link
          href="/legal#cookie-settings"
          onClick={(event) => {
            if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
            event.preventDefault();
            open();
          }}
        >
          Cookie settings
        </Link>
      </div>
    </footer>
  );
}

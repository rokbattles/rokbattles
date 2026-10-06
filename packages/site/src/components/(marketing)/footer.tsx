import { cn } from "cn";
import Image from "next/image";
import { gutter } from "./ui/layout";
import { Link } from "./ui/link";

export function MarketingFooter() {
  return (
    <footer className={cn(gutter, "flex flex-wrap items-center justify-between gap-6 py-8")}>
      <Link href="/home" className="shrink-0">
        <Image
          src="/assets/logo.svg"
          alt="ROK Battles"
          width={2104}
          height={556}
          className="h-auto w-36"
        />
      </Link>
      <div className="flex flex-wrap items-center gap-x-6 gap-y-1">
        <Link href="/legal">Legal</Link>
      </div>
    </footer>
  );
}

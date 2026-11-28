import { cn } from "cn";
import type { ReactNode } from "react";
import { DocsNavigation } from "@/components/(marketing)/docs/navigation";
import { gutter } from "@/components/(marketing)/ui/layout";

// @next-codemod-ignore Cache Components adoption: this segment temporarily allows blocking.
// Remove this opt-out after verifying the segment passes validation without it.
// See: https://nextjs.org/docs/app/guides/migrating-to-cache-components
export const instant = false;

export default function DocsLayout({ children }: { children: ReactNode }) {
  return (
    <div className="grid flex-1 content-start border-b border-white/10 md:grid-cols-[15rem_minmax(0,1fr)] md:content-stretch">
      <aside
        className={cn(gutter, "border-b border-white/10 py-6 md:border-r md:border-b-0 md:py-12")}
      >
        <DocsNavigation />
      </aside>
      <div className={cn(gutter, "min-w-0 py-10 sm:py-12 lg:px-12")}>
        <div className="mx-auto max-w-3xl">{children}</div>
      </div>
    </div>
  );
}

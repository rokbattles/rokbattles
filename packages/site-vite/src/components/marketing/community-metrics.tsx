import { cn } from "cn";
import { card, GridMarkers } from "../ui/marketing/layout";
import { bodyText } from "../ui/marketing/text";

export function CommunityMetrics() {
  return (
    <section aria-label="Community metrics" className="relative border-b border-white/10">
      <GridMarkers />
      <dl className="grid grid-cols-1 gap-px bg-white/10 sm:grid-cols-3">
        {[
          { value: "1.5K+", label: "Users" },
          { value: "40M+", label: "Battle reports" },
          { value: "400K+", label: "Governors seen" },
        ].map(({ value, label }) => (
          <div key={value} className={cn(card, "flex flex-col-reverse gap-3")}>
            <dt className={cn(bodyText, "text-zinc-400")}>{label}</dt>
            <dd className="text-3xl/none tracking-tight text-orange-400 tabular-nums sm:text-4xl lg:text-5xl">
              {value}
            </dd>
          </div>
        ))}
      </dl>
    </section>
  );
}

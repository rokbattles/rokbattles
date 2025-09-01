import { resolveNames } from "@/actions/datasets";
import type { CommanderInfo, SingleReportItem } from "@/lib/types/reports";
import BattleResultsView from "./BattleResults";
import ParticipantGear from "./ParticipantGear";

function Section({ title, children }: { title?: string; children: React.ReactNode }) {
  return (
    <section className="rounded-lg border border-white/10 bg-zinc-900/60 p-4">
      {title ? <h3 className="text-base font-semibold text-zinc-100">{title}</h3> : null}
      <div className={title ? "mt-2" : ""}>{children}</div>
    </section>
  );
}

function CommanderRow({
  info,
  names,
}: {
  info?: CommanderInfo;
  names: Record<string, string | undefined>;
}) {
  const id = info?.id;
  const nm = id ? (names[String(id)] ?? String(id)) : undefined;
  const lvl = typeof info?.level === "number" ? info.level : undefined;
  const skills = info?.skills && info.skills.length > 0 ? info.skills : undefined;

  if (!nm && !lvl && !skills) {
    return <div className="text-sm text-zinc-400">—</div>;
  }

  return (
    <div className="flex items-center justify-between">
      <div className="text-sm text-zinc-200">{nm ?? "—"}</div>
      <div className="flex items-center gap-1.5">
        {typeof lvl === "number" && (
          <span className="inline-flex items-center rounded-md px-1.5 py-0.5 text-[10px] font-semibold ring-1 ring-inset ring-white/20 text-zinc-100">
            Lv {lvl}
          </span>
        )}
        {skills && (
          <span className="inline-flex items-center rounded-md px-1.5 py-0.5 text-[10px] font-semibold ring-1 ring-inset ring-white/20 text-zinc-100">
            {skills}
          </span>
        )}
      </div>
    </div>
  );
}

export default async function ReportMain({
  item,
  locale = "en",
}: {
  item?: SingleReportItem;
  locale?: "en" | "es" | "kr";
}) {
  if (!item)
    return (
      <div className="max-w-6xl mx-auto">
        <h1 className="text-xl font-semibold text-zinc-100">Report</h1>
        <p className="mt-2 text-sm text-zinc-400">No item selected.</p>
      </div>
    );

  const self = item.report?.self;
  const enemy = item.report?.enemy;

  const leftName = self?.player_name ?? "Unknown";
  const rightName = enemy?.player_name ?? "Unknown";

  const ids: string[] = [];
  if (self?.primary_commander?.id) ids.push(String(self.primary_commander.id));
  if (self?.secondary_commander?.id) ids.push(String(self.secondary_commander.id));
  if (enemy?.primary_commander?.id) ids.push(String(enemy.primary_commander.id));
  if (enemy?.secondary_commander?.id) ids.push(String(enemy.secondary_commander.id));
  const nameMap = ids.length > 0 ? await resolveNames("commanders", ids, locale) : {};

  return (
    <div className="max-w-6xl mx-auto space-y-6">
      <BattleResultsView
        data={item.report?.battle_results}
        leftName={leftName}
        rightName={rightName}
      />
      <div className="grid gap-4 md:grid-cols-2">
        <Section>
          <div className="text-sm text-zinc-300">
            {self?.alliance_tag ? `[${self.alliance_tag}] ` : ""}
            {leftName}
          </div>
          <div className="mt-1 space-y-0.5">
            <CommanderRow info={self?.primary_commander} names={nameMap} />
            <CommanderRow info={self?.secondary_commander} names={nameMap} />
          </div>
          <div className="mt-3">
            <ParticipantGear participant={self} locale={locale} />
          </div>
        </Section>
        <Section>
          <div className="text-sm text-zinc-300">
            {enemy?.alliance_tag ? `[${enemy.alliance_tag}] ` : ""}
            {rightName}
          </div>
          <div className="mt-1 space-y-0.5">
            <CommanderRow info={enemy?.primary_commander} names={nameMap} />
            <CommanderRow info={enemy?.secondary_commander} names={nameMap} />
          </div>
          <div className="mt-3">
            <ParticipantGear participant={enemy} locale={locale} />
          </div>
        </Section>
      </div>
    </div>
  );
}

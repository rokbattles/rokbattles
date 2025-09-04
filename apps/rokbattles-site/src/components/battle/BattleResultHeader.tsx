import type { ParticipantInfo } from "@/lib/types/reports";

type Props = {
  self: ParticipantInfo;
  enemy: ParticipantInfo;
};

export async function BattleResultHeader({ self, enemy }: Props) {
  return (
    <header className="mb-4">
      <div className="grid grid-cols-[1fr_auto_1fr] items-center gap-3">
        <div className="flex items-center gap-3">
          <div className="h-9 w-9 overflow-hidden rounded-full ring-2 ring-blue-400/50">
            <div className="h-full w-full bg-blue-500" />
          </div>
          <div>
            <div className="text-sm font-semibold text-zinc-100">{self.player_name}</div>
            <div className="text-[11px] text-zinc-400">{self.alliance_tag}</div>
          </div>
        </div>
        <div className="relative flex items-center justify-center">
          <span className="rounded-full bg-zinc-900 px-3 py-1 text-xs font-bold text-zinc-200 ring-1 ring-white/10">
            VS
          </span>
        </div>
        <div className="flex items-center justify-end gap-3">
          <div className="text-right">
            <div className="text-sm font-semibold text-zinc-100">{enemy.player_name}</div>
            <div className="text-[11px] text-zinc-400">{enemy.alliance_tag}</div>
          </div>
          <div className="h-9 w-9 overflow-hidden rounded-full ring-2 ring-red-400/50">
            <div className="h-full w-full bg-red-500" />
          </div>
        </div>
      </div>
      <div className="mt-3 grid grid-cols-[1fr_1fr] gap-3">
        <div className="h-0.5 rounded-full bg-blue-500" />
        <div className="h-0.5 rounded-full bg-red-500" />
      </div>
    </header>
  );
}

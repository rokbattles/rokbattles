import { useExtracted } from "next-intl";
import { CommanderListbox } from "@/components/combat-lab/commander-listbox";
import { Text } from "@/components/ui/text";
import type { CombatLabCommanderOption } from "@/lib/combat-lab/commanders";

type CommanderPairingFormProps = {
  commanderOptions: CombatLabCommanderOption[];
  primaryCommanderId: number;
  secondaryCommanderId: number;
  onPrimaryCommanderChange: (value: number) => void;
  onSecondaryCommanderChange: (value: number) => void;
};

export function CommanderPairingForm({
  commanderOptions,
  primaryCommanderId,
  secondaryCommanderId,
  onPrimaryCommanderChange,
  onSecondaryCommanderChange,
}: CommanderPairingFormProps) {
  const t = useExtracted();
  const commandersMatch = primaryCommanderId === secondaryCommanderId;

  return (
    <div className="space-y-4">
      <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-[minmax(12rem,18rem)_minmax(12rem,18rem)]">
        <CommanderListbox
          ariaLabel={t("Primary commander")}
          commanderOptions={commanderOptions}
          label={t("Primary commander")}
          onChange={onPrimaryCommanderChange}
          value={primaryCommanderId}
        />
        <CommanderListbox
          ariaLabel={t("Secondary commander")}
          commanderOptions={commanderOptions}
          label={t("Secondary commander")}
          onChange={onSecondaryCommanderChange}
          value={secondaryCommanderId}
        />
      </div>
      {commandersMatch ? (
        <Text>{t("Choose two different legendary commanders before loading data.")}</Text>
      ) : null}
    </div>
  );
}

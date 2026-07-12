"use client";

import { parseAsInteger, useQueryStates } from "nuqs";
import { Suspense } from "react";
import { CombatLabLoading } from "@/components/combat-lab/combat-lab-loading";
import { CombatLabPairingResults } from "@/components/combat-lab/combat-lab-pairing-results";
import { CommanderPairingForm } from "@/components/combat-lab/commander-pairing-form";
import { useClientReady } from "@/hooks/use-client-ready";
import type { CombatLabCommanderOption } from "@/lib/combat-lab/commanders";

type CombatLabContentProps = {
  commanderOptions: CombatLabCommanderOption[];
};

export function CombatLabContent({ commanderOptions }: CombatLabContentProps) {
  const clientReady = useClientReady();
  const [pairing, setPairing] = useQueryStates(
    {
      primary: parseAsInteger.withDefault(579),
      secondary: parseAsInteger.withDefault(575),
    },
    {
      clearOnDefault: false,
      history: "push",
    }
  );

  return (
    <div className="space-y-8">
      <CommanderPairingForm
        commanderOptions={commanderOptions}
        primaryCommanderId={pairing.primary}
        secondaryCommanderId={pairing.secondary}
        onPrimaryCommanderChange={(primary) =>
          setPairing({ primary, secondary: pairing.secondary })
        }
        onSecondaryCommanderChange={(secondary) =>
          setPairing({ primary: pairing.primary, secondary })
        }
      />
      {pairing.primary === pairing.secondary ? null : !clientReady ? (
        <CombatLabLoading />
      ) : (
        <Suspense fallback={<CombatLabLoading />}>
          <CombatLabPairingResults
            key={`${pairing.primary}:${pairing.secondary}`}
            primaryCommanderId={pairing.primary}
            primaryName={getCommanderOptionLabel(commanderOptions, pairing.primary)}
            secondaryCommanderId={pairing.secondary}
            secondaryName={getCommanderOptionLabel(commanderOptions, pairing.secondary)}
          />
        </Suspense>
      )}
    </div>
  );
}

function getCommanderOptionLabel(options: CombatLabCommanderOption[], id: number): string {
  return options.find((option) => option.id === id)?.name ?? id.toString();
}

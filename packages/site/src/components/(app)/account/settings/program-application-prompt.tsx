import { SettingsEmptyState } from "@/components/(app)/account/settings/settings-empty-state";
import { Button } from "@/components/ui/button";

type ProgramApplicationPromptProps = {
  program: "Content Creator" | "Developer";
};

export function ProgramApplicationPrompt({ program }: ProgramApplicationPromptProps) {
  return (
    <SettingsEmptyState
      action={
        <Button outline disabled>
          Apply to join
        </Button>
      }
    >
      Join the {program} Program. Apply to be considered for approval.
    </SettingsEmptyState>
  );
}

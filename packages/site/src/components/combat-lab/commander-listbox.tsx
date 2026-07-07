import { Listbox, ListboxLabel, ListboxOption } from "@/components/ui/listbox";
import type { CombatLabCommanderOption } from "@/lib/combat-lab/commanders";

type CommanderListboxProps = {
  ariaLabel: string;
  commanderOptions: CombatLabCommanderOption[];
  label: string;
  onChange: (value: number) => void;
  value: number;
};

export function CommanderListbox({
  ariaLabel,
  commanderOptions,
  label,
  onChange,
  value,
}: CommanderListboxProps) {
  return (
    <div className="space-y-1.5">
      <span className="block font-medium text-sm/6 text-zinc-700 dark:text-zinc-200">{label}</span>
      <Listbox<number> aria-label={ariaLabel} onChange={onChange} value={value}>
        {commanderOptions.map((option) => (
          <ListboxOption key={option.id} value={option.id}>
            <ListboxLabel>{option.name}</ListboxLabel>
          </ListboxOption>
        ))}
      </Listbox>
    </div>
  );
}

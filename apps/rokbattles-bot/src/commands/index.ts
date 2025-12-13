import { HelpCommand } from "@/commands/system/HelpCommand";
import { CommandCollection } from "@/lib/CommandHandler";

export function commands(): CommandCollection {
  const coll = new CommandCollection();

  coll.add(HelpCommand);

  return coll;
}

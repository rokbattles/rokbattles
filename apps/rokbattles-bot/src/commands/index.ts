import { HelpCommand } from "@/commands/system/HelpCommand";
import { MailcacheCommand } from "@/commands/system/MailcacheCommand";
import { CommandCollection } from "@/lib/CommandHandler";

export function commands(): CommandCollection {
  const coll = new CommandCollection();

  coll.add(HelpCommand);
  coll.add(MailcacheCommand);
  // coll.add(ReportsCommand);
  // coll.add(PairingsCommand);

  return coll;
}

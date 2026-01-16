import { PairingsCommand } from "@/commands/self/pairings-command";
import { ReportsCommand } from "@/commands/self/reports-command";
import { HelpCommand } from "@/commands/system/help-command";
import { MailcacheCommand } from "@/commands/system/mailcache-command";
import { CommandCollection } from "@/lib/command-handler";

export function commands(): CommandCollection {
  const coll = new CommandCollection();

  coll.add(HelpCommand);
  coll.add(MailcacheCommand);
  coll.add(ReportsCommand);
  coll.add(PairingsCommand);

  return coll;
}

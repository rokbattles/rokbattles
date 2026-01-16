import { PairingsCommand } from "@/commands/self/pairings-command";
import { ReportsCommand } from "@/commands/self/reports-command";
import { HelpCommand } from "@/commands/system/help-command";
import { MailcacheCommand } from "@/commands/system/mailcache-command";
import type { BaseClient } from "@/lib/base-client";
import { CommandCollection } from "@/lib/command-handler";

export function commands(): CommandCollection<BaseClient> {
  const coll = new CommandCollection<BaseClient>();

  coll.add(HelpCommand);
  coll.add(MailcacheCommand);
  coll.add(ReportsCommand);
  coll.add(PairingsCommand);

  return coll;
}

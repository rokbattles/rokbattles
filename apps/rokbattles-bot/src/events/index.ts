import { SlashCommandHandler } from "@/events/slash-command-handler";
import type { BaseClient } from "@/lib/base-client";
import { EventCollection } from "@/lib/event-handler";

export function events(): EventCollection<BaseClient> {
  const coll = new EventCollection<BaseClient>();

  coll.add("interactionCreate", SlashCommandHandler);

  return coll;
}

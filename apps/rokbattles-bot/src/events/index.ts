import { SlashCommandHandler } from "@/events/slash-command-handler";
import { EventCollection } from "@/lib/event-handler";

export function events(): EventCollection {
  const coll = new EventCollection();

  coll.add("interactionCreate", SlashCommandHandler);

  return coll;
}

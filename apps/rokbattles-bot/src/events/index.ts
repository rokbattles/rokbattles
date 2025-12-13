import { SlashCommandHandler } from "@/events/SlashCommandHandler";
import { EventCollection } from "@/lib/EventHandler";

export function events(): EventCollection {
  const coll = new EventCollection();

  coll.add("interactionCreate", SlashCommandHandler);

  return coll;
}

import { Client, type ClientOptions } from "discord.js";
import { commands } from "@/commands";
import { events } from "@/events";
import { registerEvents } from "@/lib/event-handler";

export class BaseClient extends Client {
  commands = commands();

  constructor(options: ClientOptions) {
    super(options);

    const eventColl = events();
    console.log(`Successfully loaded ${eventColl.size} events`);
    registerEvents(this, eventColl);
  }
}

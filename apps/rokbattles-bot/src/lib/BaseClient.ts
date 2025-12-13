import { Client, type ClientOptions } from "discord.js";
import { commands } from "@/commands";
import { events } from "@/events";
import { registerEvents } from "@/lib/EventHandler";

export class BaseClient extends Client {
  public commands = commands();

  constructor(options: ClientOptions) {
    super(options);

    console.log(`Successfully loaded ${events().size} events`);
    registerEvents(this, events());
  }
}

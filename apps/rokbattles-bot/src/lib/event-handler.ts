import { type Client, type ClientEvents, Collection } from "discord.js";

export type EventHandler<C extends Client, E extends keyof ClientEvents> = (
  client: C,
  ...args: ClientEvents[E]
) => Promise<unknown> | unknown;

export class EventCollection<C extends Client = Client> extends Collection<
  keyof ClientEvents,
  EventHandler<C, keyof ClientEvents>[]
> {
  add<E extends keyof ClientEvents>(event: E, handler: EventHandler<C, E>) {
    const item = this.get(event) || [];
    item.push(handler as EventHandler<C, keyof ClientEvents>);
    this.set(event, item);
  }
}

export function registerEvents<C extends Client>(
  client: C,
  collection: EventCollection<C>
) {
  collection.forEach((handlers, event) => {
    const method = event === "ready" ? "once" : "on";

    client[method](event, (...args) => {
      for (const handler of handlers) {
        handler(client, ...args);
      }
    });
  });
}

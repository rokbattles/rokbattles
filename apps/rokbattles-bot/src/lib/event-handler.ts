import { type Client, type ClientEvents, Collection } from "discord.js";

export type EventHandler<C extends Client, E extends keyof ClientEvents> = (
  client: C,
  ...args: ClientEvents[E]
) => Promise<unknown> | unknown;

// biome-ignore lint/suspicious/noExplicitAny: ignore
export class EventCollection extends Collection<
  keyof ClientEvents,
  EventHandler<any, any>[]
> {
  // biome-ignore lint/suspicious/noExplicitAny: ignore
  add(event: keyof ClientEvents, handler: EventHandler<any, any>) {
    const item = this.get(event) || [];
    item.push(handler);
    this.set(event, item);
  }
}

export function registerEvents<C extends Client>(
  client: C,
  collection: EventCollection
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

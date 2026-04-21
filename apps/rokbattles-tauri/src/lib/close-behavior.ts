export type CloseBehavior = "ask" | "minimize_to_tray" | "quit";
export type CloseChoice = Exclude<CloseBehavior, "ask">;

export function parseCloseBehavior(value: unknown): CloseBehavior {
  if (value === "ask" || value === "minimize_to_tray" || value === "quit") {
    return value;
  }

  return "ask";
}

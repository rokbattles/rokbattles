export type CloseBehavior = "minimize_to_tray" | "quit";

export function parseCloseBehavior(value: unknown): CloseBehavior {
  if (value === "minimize_to_tray" || value === "quit") {
    return value;
  }

  return "minimize_to_tray";
}

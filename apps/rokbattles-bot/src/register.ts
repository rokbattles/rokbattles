import { commands } from "@/commands";
import { registerGlobalCommands } from "@/lib/CommandHandler";

(async () => {
  try {
    const options = [...commands().values()].map((cmd) => cmd.options);
    await registerGlobalCommands(
      process.env.DISCORD_TOKEN!,
      process.env.DISCORD_APPLICATION_ID!,
      options
    );
  } catch {
    process.exit(1);
  }
})();

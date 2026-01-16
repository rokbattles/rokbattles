import { commands } from "@/commands";
import { registerGlobalCommands } from "@/lib/command-handler";

(async () => {
  try {
    if (!(process.env.DISCORD_TOKEN && process.env.DISCORD_APPLICATION_ID)) {
      console.error("DISCORD_TOKEN and DISCORD_APPLICATION_ID must be set");
      process.exit(1);
    }

    const options = [...commands().values()].map((cmd) => cmd.options);
    await registerGlobalCommands(
      process.env.DISCORD_TOKEN,
      process.env.DISCORD_APPLICATION_ID,
      options
    );
  } catch {
    process.exit(1);
  }
})();

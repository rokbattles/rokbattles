import { unregisterGlobalCommands } from "@/lib/command-handler";

(async () => {
  try {
    if (!(process.env.DISCORD_TOKEN && process.env.DISCORD_APPLICATION_ID)) {
      console.error("DISCORD_TOKEN and DISCORD_APPLICATION_ID must be set");
      process.exit(1);
    }

    await unregisterGlobalCommands(
      process.env.DISCORD_TOKEN,
      process.env.DISCORD_APPLICATION_ID
    );
  } catch {
    process.exit(1);
  }
})();

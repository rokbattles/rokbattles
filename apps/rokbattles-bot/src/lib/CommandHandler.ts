import {
  type ApplicationCommand,
  type ApplicationCommandData,
  type AutocompleteInteraction,
  type ChatInputCommandInteraction,
  type Client,
  Collection,
  REST,
  Routes,
} from "discord.js";

export type CommandHandler<C extends Client> = {
  chatInput: (client: C, interaction: ChatInputCommandInteraction) => Promise<unknown> | unknown;
  autocomplete?: (client: C, interaction: AutocompleteInteraction) => Promise<unknown> | unknown;
  options: ApplicationCommandData;
};

// biome-ignore lint/suspicious/noExplicitAny: ignore
export class CommandCollection extends Collection<string, CommandHandler<any>> {
  // biome-ignore lint/suspicious/noExplicitAny: ignore
  add(command: CommandHandler<any>) {
    super.set(command.options.name.toLowerCase(), command);
  }
}

export async function registerGlobalCommands(
  token: string,
  applicationId: string,
  commands: ApplicationCommandData[]
) {
  const rest = new REST().setToken(token);

  try {
    await rest.put(Routes.applicationCommands(applicationId), {
      body: commands,
    });
    console.log(`Successfully registered ${commands.length} application commands`);
  } catch (e) {
    console.error(e);
    process.exit(1);
  }
}

export async function unregisterGlobalCommands(token: string, applicationId: string) {
  const rest = new REST().setToken(token);

  try {
    const commands = (await rest.get(
      Routes.applicationCommands(applicationId)
    )) as ApplicationCommand[];

    for (const command of commands) {
      await rest.delete(Routes.applicationCommand(applicationId, command.id));
    }

    console.log(`Successfully deleted ${commands.length} application commands`);
  } catch (e) {
    console.error(e);
    process.exit(1);
  }
}

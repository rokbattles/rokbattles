use twilight_model::application::{
    command::{CommandOption, CommandOptionType},
    interaction::application_command::{CommandData, CommandDataOption, CommandOptionValue},
};

use crate::framework::{Command, CommandContext, CommandMetadataBuilder};

pub fn mailcache_command() -> Command {
    Command::new(
        "mailcache",
        CommandMetadataBuilder::new(
            "mailcache",
            "Guide to locate your Rise of Kingdoms mailcache directory",
        )
        .with_options(vec![
            subcommand_option("windows", "Find your mailcache directory on Windows"),
            subcommand_option("macos", "Find your mailcache directory on macOS"),
        ])
        .build(),
        |ctx: CommandContext| async move {
            let content = match ctx.command_data() {
                Some(command_data) => content_for_command_data(command_data),
                None => fallback_content(),
            };
            ctx.reply(content).await
        },
    )
}

fn subcommand_option(name: &str, description: &str) -> CommandOption {
    CommandOption {
        autocomplete: None,
        channel_types: None,
        choices: None,
        description: description.to_owned(),
        description_localizations: None,
        kind: CommandOptionType::SubCommand,
        max_length: None,
        max_value: None,
        min_length: None,
        min_value: None,
        name: name.to_owned(),
        name_localizations: None,
        options: None,
        required: None,
    }
}

fn content_for_command_data(command_data: &CommandData) -> &'static str {
    match extract_subcommand(command_data.options.as_slice()) {
        Some("windows") => windows_content(),
        Some("macos") => macos_content(),
        _ => fallback_content(),
    }
}

fn extract_subcommand(options: &[CommandDataOption]) -> Option<&str> {
    options.iter().find_map(|option| match option.value {
        CommandOptionValue::SubCommand(_) => Some(option.name.as_str()),
        _ => None,
    })
}

fn fallback_content() -> &'static str {
    "Please select a platform subcommand: `/mailcache windows` or `/mailcache macos`."
}

fn windows_content() -> &'static str {
    r#"## Mailcache Location (Windows)
If you haven't changed the default install location of Rise of Kingdoms, the mailcache folder is usually here:
`C:\Program Files (x86)\Rise of Kingdoms\Rise of Kingdoms Game\save\mailcache`

### What if it's not there?
You can find your current installation path using the Rise of Kingdoms launcher:

1. Open the Rise of Kingdoms launcher (do not launch the game)
2. Click the ⚙️ icon in the top-right corner
3. Select "Game Resources" from the sidebar
4. Under "Local Files," find "Current installation path"

From that base directory, navigate to:
`Rise of Kingdoms Game\save\mailcache`"#
}

fn macos_content() -> &'static str {
    r#"## Mailcache Location (macOS)
_Tip: You may need to show hidden directories in Finder with `Cmd + Shift + .`_

1. Open Finder
2. Go to the `/Library/Containers` directory
3. Find the Rise of Kingdoms container directory called "RiseOfKingdoms"
4. Open that directory, then navigate to: `Data/Documents/mailcache`

_Note: Inside of ROK Battles desktop app, it will display the App ID instead of RiseOfKingdoms._"#
}

#[cfg(test)]
mod tests {
    use twilight_model::application::interaction::application_command::{
        CommandDataOption, CommandOptionValue,
    };

    use super::{
        content_for_command_data, fallback_content, macos_content, mailcache_command,
        windows_content,
    };

    fn command_data_with_subcommand(
        name: &str,
    ) -> twilight_model::application::interaction::application_command::CommandData {
        twilight_model::application::interaction::application_command::CommandData {
            guild_id: None,
            id: twilight_model::id::Id::new(1),
            name: "mailcache".to_owned(),
            kind: twilight_model::application::command::CommandType::ChatInput,
            options: vec![CommandDataOption {
                name: name.to_owned(),
                value: CommandOptionValue::SubCommand(Vec::new()),
            }],
            resolved: None,
            target_id: None,
        }
    }

    #[test]
    fn metadata_contains_two_subcommands() {
        let command = mailcache_command();
        assert_eq!(command.metadata.name, "mailcache");
        assert_eq!(command.metadata.options.len(), 2);
        assert_eq!(command.metadata.options[0].name, "windows");
        assert_eq!(command.metadata.options[1].name, "macos");
    }

    #[test]
    fn windows_subcommand_maps_to_windows_content() {
        let data = command_data_with_subcommand("windows");
        assert_eq!(content_for_command_data(&data), windows_content());
    }

    #[test]
    fn macos_subcommand_maps_to_macos_content() {
        let data = command_data_with_subcommand("macos");
        assert_eq!(content_for_command_data(&data), macos_content());
    }

    #[test]
    fn unknown_subcommand_uses_fallback_content() {
        let data = command_data_with_subcommand("linux");
        assert_eq!(content_for_command_data(&data), fallback_content());
    }
}

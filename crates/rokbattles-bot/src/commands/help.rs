use crate::framework::{Command, CommandContext, CommandMetadataBuilder};

pub fn help_command() -> Command {
    Command::new(
        "help",
        CommandMetadataBuilder::new("help", "Information about ROK Battles and relevant links")
            .build(),
        |ctx: CommandContext| async move { ctx.reply(help_content()).await },
    )
}

fn help_content() -> &'static str {
    r#"## ROK Battles
A community-driven platform for sharing, exploring, and analyzing battle reports and viewing trends in Rise of Kingdoms.

### Relevant Links
* Platform: <https://platform.rokbattles.com>
* Desktop app: <https://platform.rokbattles.com/desktop-app>
* Support: <https://platform.rokbattles.com/discord>"#
}

#[cfg(test)]
mod tests {
    use super::{help_command, help_content};

    #[test]
    fn help_metadata_is_stable() {
        let command = help_command();
        assert_eq!(command.metadata.name, "help");
        assert_eq!(
            command.metadata.description,
            "Information about ROK Battles and relevant links"
        );
        assert!(command.metadata.options.is_empty());
    }

    #[test]
    fn help_content_has_expected_heading() {
        assert!(help_content().starts_with("## ROK Battles"));
    }
}

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = mail_cli_legacy::Cli::parse();
    mail_cli_legacy::run(cli).await
}

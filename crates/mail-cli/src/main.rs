use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = mail_cli::Cli::parse();
    mail_cli::run(cli).await
}

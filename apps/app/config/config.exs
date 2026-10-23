# This file is responsible for configuring your application
# and its dependencies with the aid of the Config module.
#
# This configuration file is loaded before any dependency and
# is restricted to this project.

# General application configuration
import Config

config :rokbattles,
  namespace: ROKBattles,
  ecto_repos: [ROKBattles.Repo, ROKBattles.ClickHouseRepo],
  generators: [timestamp_type: :utc_datetime]

# Configure background jobs
config :rokbattles, Oban,
  repo: ROKBattles.Repo,
  queues: [default: 10],
  lifeline: [rescue_after: {2, :hours}],
  pruner: [max_age: {1, :day}]

# Enable Sentry only through production runtime configuration.
config :sentry,
  dsn: nil,
  in_app_otp_apps: [:rokbattles]

# Configure the endpoint
config :rokbattles, ROKBattlesWeb.Endpoint,
  url: [host: "localhost"],
  adapter: Bandit.PhoenixAdapter,
  render_errors: [
    formats: [html: ROKBattlesWeb.ErrorHTML, json: ROKBattlesWeb.ErrorJSON],
    layout: false
  ],
  pubsub_server: ROKBattles.PubSub,
  live_view: [signing_salt: "Xp6cuf5e"]

# Configure LiveView
config :phoenix_live_view,
  # the attribute set on all root tags. Used for Phoenix.LiveView.ColocatedCSS.
  root_tag_attribute: "phx-r"

# Configure the mailer
#
# By default it uses the "Local" adapter which stores the emails
# locally. You can see the emails in your browser, at "/dev/mailbox".
#
# For production it's recommended to configure a different adapter
# at the `config/runtime.exs`.
config :rokbattles, ROKBattles.Mailer, adapter: Swoosh.Adapters.Local

# Configure esbuild (the version is required)
config :esbuild,
  version: "0.25.4",
  rokbattles: [
    args:
      ~w(js/app.js --bundle --target=es2022 --outdir=../priv/static/assets/js --external:/fonts/* --external:/images/* --alias:@=.),
    cd: Path.expand("../assets", __DIR__),
    env: %{"NODE_PATH" => [Path.expand("../deps", __DIR__), Mix.Project.build_path()]}
  ]

# Configure tailwind (the version is required)
config :tailwind,
  version: "4.3.3",
  rokbattles: [
    args: ~w(
      --input=assets/css/app.css
      --output=priv/static/assets/css/app.css
    ),
    cd: Path.expand("..", __DIR__),
    env: %{"NODE_PATH" => [Path.expand("../deps", __DIR__), Mix.Project.build_path()]}
  ]

# Configure Elixir's Logger
config :logger, :default_formatter,
  format: "$time $metadata[$level] $message\n",
  metadata: [:request_id]

# Use Jason for JSON parsing in Phoenix
config :phoenix, :json_library, Jason

# Import environment specific config. This must remain at the bottom
# of this file so it overrides the configuration defined above.
import_config "#{config_env()}.exs"

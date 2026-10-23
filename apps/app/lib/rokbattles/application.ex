defmodule ROKBattles.Application do
  # See https://elixir.hexdocs.pm/Application.html
  # for more information on OTP Applications
  @moduledoc false

  use Application

  @impl true
  def start(_type, _args) do
    children = [
      ROKBattlesWeb.Telemetry,
      ROKBattles.Repo,
      ROKBattles.ClickHouseRepo,
      {DNSCluster, query: Application.get_env(:rokbattles, :dns_cluster_query) || :ignore},
      {Phoenix.PubSub, name: ROKBattles.PubSub},
      # Start a worker by calling: ROKBattles.Worker.start_link(arg)
      # {ROKBattles.Worker, arg},
      # Start to serve requests, typically the last entry
      ROKBattlesWeb.Endpoint
    ]

    # See https://elixir.hexdocs.pm/Supervisor.html
    # for other strategies and supported options
    opts = [strategy: :one_for_one, name: ROKBattles.Supervisor]
    Supervisor.start_link(children, opts)
  end

  # Tell Phoenix to update the endpoint configuration
  # whenever the application is updated.
  @impl true
  def config_change(changed, _new, removed) do
    ROKBattlesWeb.Endpoint.config_change(changed, removed)
    :ok
  end
end

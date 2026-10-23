defmodule ROKBattles.ClickHouseRepo do
  use Ecto.Repo,
    otp_app: :rokbattles,
    adapter: Ecto.Adapters.ClickHouse
end

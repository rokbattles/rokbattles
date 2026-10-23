defmodule ROKBattles.RateLimit do
  @moduledoc """
  In-memory rate limiter with counters local to each application node.

  Limits are supplied by callers when needed; none are enforced automatically.
  Counters reset when the limiter restarts.
  """

  use Hammer, backend: :ets
end

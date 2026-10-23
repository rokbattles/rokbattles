defmodule ROKBattlesWeb.PageController do
  use ROKBattlesWeb, :controller

  def home(conn, _params) do
    render(conn, :home)
  end
end

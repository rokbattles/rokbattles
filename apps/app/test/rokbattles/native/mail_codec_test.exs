defmodule ROKBattles.Native.MailCodecTest do
  use ExUnit.Case, async: true

  alias ROKBattles.Native.MailCodec

  doctest MailCodec

  @fixture Path.expand(
             "../../../../../samples/Battle/Persistent.Mail.10121648172261838131",
             __DIR__
           )
  @known_mail Base.decode16!("FFCAAEDA9C54706D580504020000006F6B0101FF")

  test "decodes the known mail header, checksum, and payload" do
    assert MailCodec.decode(@known_mail) == {:ok, %{"ok" => true}}
  end

  test "encodes the known mail header, checksum, and payload" do
    assert MailCodec.encode(%{"ok" => true}) == {:ok, @known_mail}
  end

  test "decodes a real mail sample into Elixir values and re-encodes it" do
    assert {:ok, decoded} = @fixture |> File.read!() |> MailCodec.decode()
    assert decoded["id"] == "10121648172261838131"
    assert is_map(decoded["body"]["content"])
    assert {:ok, encoded} = MailCodec.encode(decoded)
    assert MailCodec.decode(encoded) == {:ok, decoded}
  end

  test "round-trips nested maps, lists, booleans, UTF-8 strings, and numbers" do
    value = %{
      "id" => "123",
      "unread" => false,
      "body" => %{"text" => "Hello 世界 🌍", "values" => [true, -42, 1.25, ""]},
      "attachments" => []
    }

    assert {:ok, binary} = MailCodec.encode(value)
    assert MailCodec.decode(binary) == {:ok, value}
  end

  test "preserves the codec's nil, empty-map, and whole-float conversions" do
    value = %{"omitted" => nil, "empty" => %{}, "whole" => 1.0}
    assert {:ok, binary} = MailCodec.encode(value)
    assert MailCodec.decode(binary) === {:ok, %{"empty" => [], "whole" => 1}}
    assert {:error, _} = MailCodec.encode(nil)
    assert {:error, _} = MailCodec.encode([nil])
  end

  test "preserves signed and unsigned integers exactly when representable as floats" do
    value = [-9_223_372_036_854_775_808, 9_223_372_036_854_775_808]
    assert {:ok, binary} = MailCodec.encode(value)
    assert MailCodec.decode(binary) === {:ok, value}
  end

  test "returns errors for malformed mail binaries and wrong input types" do
    <<_marker, rest::binary>> = @known_mail
    <<prefix::binary-size(19), _terminator>> = @known_mail

    for invalid <- [<<>>, "not mail", <<0, rest::binary>>, <<prefix::binary, 0>>, nil, %{}, []] do
      assert {:error, reason} = MailCodec.decode(invalid)
      assert is_binary(reason)
    end

    assert {:ok, %{"ok" => true}} = MailCodec.decode(@known_mail)
  end

  test "rejects unsupported Elixir values without raising or silently changing them" do
    for invalid <- [
          %{id: "123"},
          %{1 => "value"},
          %{"bad" => <<255>>},
          %{<<255>> => true},
          :unexpected,
          {1, 2},
          [true | false],
          <<1::1>>,
          self(),
          18_446_744_073_709_551_616,
          -9_223_372_036_854_775_809
        ] do
      assert {:error, reason} = MailCodec.encode(invalid)
      assert is_binary(reason)
    end
  end

  test "accepts 128 nested tables and rejects deeper input before native recursion grows" do
    at_limit = Enum.reduce(1..128, true, fn _, value -> [value] end)
    assert {:ok, binary} = MailCodec.encode(at_limit)
    assert MailCodec.decode(binary) == {:ok, at_limit}
    assert {:error, "table nesting exceeds max depth of 128"} = MailCodec.encode([at_limit])

    too_deep = Enum.reduce(1..10_000, true, fn _, value -> [value] end)
    assert {:error, "table nesting exceeds max depth of 128"} = MailCodec.encode(too_deep)
  end

  test "calls from multiple processes do not share codec state" do
    supervisor = start_supervised!(Task.Supervisor)

    tasks =
      for id <- 1..8 do
        Task.Supervisor.async(supervisor, fn ->
          value = %{"id" => id, "unread" => true}
          {:ok, binary} = MailCodec.encode(value)
          {MailCodec.decode(binary), value}
        end)
      end

    for task <- tasks do
      assert {{:ok, value}, value} = Task.await(task)
    end
  end
end

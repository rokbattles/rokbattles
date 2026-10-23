defmodule ROKBattles.Native.MailCodec do
  @moduledoc """
  Reads and writes complete `Persistent.Mail` binaries using the shared Rust codec.

  Decoded objects have string keys, arrays are lists, and scalar values are UTF-8
  strings, numbers, or booleans. Encoding accepts the same values, plus `nil` in
  map fields. It does not accept atom keys, structs, tuples, or arbitrary binaries
  as string values. Integers must fit a signed or unsigned 64-bit integer.

  The format omits map fields containing `nil`, rejects `nil` at the root or in
  lists, and decodes empty maps as `[]`. Numbers are stored as 64-bit floats, so
  large integers may lose precision and whole floats decode as integers. Tables
  may nest up to 128 levels. Re-encoding preserves the codec's value semantics,
  but does not guarantee identical bytes.

  Both functions run on dirty CPU schedulers and return `{:error, reason}` for
  invalid input. Mix builds the native library from source; building requires
  the repository's Rust toolchain and the shared crates in this checkout.

  ## Examples

      iex> {:ok, binary} = ROKBattles.Native.MailCodec.encode(%{"id" => "123", "unread" => true})
      iex> ROKBattles.Native.MailCodec.decode(binary)
      {:ok, %{"id" => "123", "unread" => true}}
  """

  @external_resource Path.expand("../../../../../Cargo.toml", __DIR__)

  use Rustler,
    otp_app: :rokbattles,
    crate: "rokbattles_mail_codec_nif"

  @type value :: boolean() | number() | String.t() | [value()] | %{String.t() => value() | nil}
  @type result(value) :: {:ok, value} | {:error, String.t()}

  @doc """
  Validates the file header and checksum, then decodes the binary into Elixir values.
  """
  @spec decode(binary()) :: result(value())
  def decode(_binary), do: :erlang.nif_error(:nif_not_loaded)

  @doc """
  Encodes a value into a mail binary, including its header and checksum.
  """
  @spec encode(value()) :: result(binary())
  def encode(_value), do: :erlang.nif_error(:nif_not_loaded)
end

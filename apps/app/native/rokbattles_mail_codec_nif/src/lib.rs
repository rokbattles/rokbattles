//! Rustler bindings for the shared persistent mail codec.

use rustler::{Binary, Env, OwnedBinary, Term};

mod value;

#[rustler::nif(schedule = "DirtyCpu")]
fn decode<'a>(env: Env<'a>, input: Term<'a>) -> Result<Term<'a>, String> {
    let bytes = input.decode::<Binary>().map_err(|_| "expected a mail binary".to_owned())?;
    let decoded =
        rokbattles_mail_codec::decode(bytes.as_slice()).map_err(|error| error.to_string())?;
    rustler::serde::to_term(env, &decoded).map_err(|error| error.to_string())
}

#[rustler::nif(schedule = "DirtyCpu")]
fn encode<'a>(env: Env<'a>, input: Term<'a>) -> Result<Binary<'a>, String> {
    let value = value::from_term(input, 0)?;
    let bytes = rokbattles_mail_codec::encode(&value).map_err(|error| error.to_string())?;
    let mut binary = OwnedBinary::new(bytes.len())
        .ok_or_else(|| "could not allocate the encoded mail binary".to_owned())?;
    binary.as_mut_slice().copy_from_slice(&bytes);
    Ok(binary.release(env))
}

rustler::init!("Elixir.ROKBattles.Native.MailCodec");

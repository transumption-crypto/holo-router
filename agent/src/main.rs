use zerotier::Identity; 
use holochain_common::DEFAULT_PASSPHRASE;
use holochain_conductor_api::key_loaders::mock_passphrase_manager;
use holochain_conductor_api::keystore::Keystore;

use ed25519_dalek::Keypair;
use failure::Error;
use serde::*;

use std::convert::{TryFrom, TryInto};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, path::PathBuf};

/// Additional data for Ed25519ph domain separation.
///
/// In order to prove ownership, Holo Router uses signatures made by secret keys that were designed
/// to sign something entirely different. This makes sure that we never accidently sign a Holofuel
/// transaction.
pub const CONTEXT: &[u8] = b"holo-router v1 with Holochain + ZeroTier signature";

fn serialize_instant<S>(t: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u128(t.duration_since(UNIX_EPOCH).unwrap().as_millis())
}

#[derive(Debug, Serialize)]
struct Payload {
    #[serde(serialize_with = "serialize_instant")]
    instant: SystemTime,
    // holochain_public_key: ed25519_dalek::PublicKey,
    zerotier_address: zerotier::Address
}

fn main() -> Result<(), Error> {
    let identity = Identity::try_from(&fs::read_to_string("/var/lib/zerotier-one/identity.secret")?[..])?;

    let address = identity.address.clone();

    // See ed25519_dalek::Keypair docs. This can be used to sign the payload.
    let keypair: Keypair = identity.try_into()?;

    let passphrase_manager = mock_passphrase_manager(DEFAULT_PASSPHRASE.into());

    // Holochain keystore (somehow figure out how to extract ed25519 key from it)
    let keystore =
        Keystore::new_from_file(PathBuf::from("holo-keystore"), passphrase_manager, None)?;

    let payload = Payload {
        instant: SystemTime::now(),
        zerotier_address: address
    };

    println!("{}", serde_json::to_string(&payload)?);

    Ok(())
}

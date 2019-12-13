use holochain_common::DEFAULT_PASSPHRASE;
use holochain_conductor_api::key_loaders::mock_passphrase_manager;
use holochain_conductor_api::keystore::{Keystore, PRIMARY_KEYBUNDLE_ID};

use ed25519_dalek::{Keypair, PublicKey, Signature};
use failure::Error;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::Client;
use serde::*;
use zerotier::Identity;

use std::convert::{TryFrom, TryInto};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{fs, path::PathBuf};

fn encode_holochain_agent_id<S>(public_key: &PublicKey, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&base36::encode(&public_key.to_bytes()[..]))
}

fn encode_instant<S>(time: &SystemTime, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u128(time.duration_since(UNIX_EPOCH).unwrap().as_millis())
}

fn header_signature(signature: &Signature) -> HeaderValue {
    base64::encode(&signature.to_bytes()[..]).parse().unwrap()
}

#[derive(Debug, Serialize)]
struct Payload {
    #[serde(serialize_with = "encode_instant")]
    instant: SystemTime,
    #[serde(serialize_with = "encode_holochain_agent_id")]
    holochain_agent_id: PublicKey,
    zerotier_address: zerotier::Address,
}

fn holochain_keypair(path: PathBuf) -> Result<Keypair, Error> {
    let passphrase_manager = mock_passphrase_manager(DEFAULT_PASSPHRASE.into());
    let mut keystore = Keystore::new_from_file(path, passphrase_manager, None)?;

    let keybundle = keystore.get_keybundle(PRIMARY_KEYBUNDLE_ID)?;

    let mut secret_key_secbuf = keybundle.sign_keys.private;
    let secret_key_bytes = &**secret_key_secbuf.read_lock();

    Ok(Keypair::from_bytes(&secret_key_bytes[..])?)
}

fn main() -> Result<(), Error> {
    let holochain_keypair = holochain_keypair(PathBuf::from("holo-keystore"))?;

    let zerotier_identity =
        Identity::try_from(&fs::read_to_string("/var/lib/zerotier-one/identity.secret")?[..])?;
    let zerotier_address = zerotier_identity.address.clone();
    let zerotier_keypair: Keypair = zerotier_identity.try_into()?;

    let payload = Payload {
        instant: SystemTime::now(),
        holochain_agent_id: holochain_keypair.public,
        zerotier_address: zerotier_address,
    };

    let payload_bytes = serde_json::to_vec(&payload)?;

    let holochain_signature = holochain_keypair.sign(&payload_bytes[..]);
    let zerotier_signature = zerotier_keypair.sign(&payload_bytes[..]);

    let mut headers = HeaderMap::new();

    headers.insert(
        HeaderName::from_static("x-holochain-signature"),
        header_signature(&holochain_signature),
    );

    headers.insert(
        HeaderName::from_static("x-zerotier-signature"),
        header_signature(&zerotier_signature),
    );

    Client::new()
        .post("https://router-registry.holo.host/v1/update")
        .headers(headers)
        .body(payload_bytes)
        .send()?;

    Ok(())
}

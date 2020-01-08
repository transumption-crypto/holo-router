use failure::*;
use futures::future;
use log::{log, log_enabled, Level};
use tokio::net::{TcpListener, TcpStream};
use uuid::Uuid;

// See: https://tls.ulfheim.net
use rustls::internal::msgs::codec::{Codec, Reader};
use rustls::internal::msgs::enums::{ContentType, ProtocolVersion};
use rustls::internal::msgs::handshake::{
    HandshakeMessagePayload, HandshakePayload, ServerNamePayload,
};

use std::cell::RefCell;
use std::io::Write;

const TLS_HANDSHAKE_MAX_LENGTH: usize = 2048;
const TLS_RECORD_HEADER_LENGTH: usize = 5;

async fn peek(stream: &mut TcpStream, size: usize) -> Fallible<Vec<u8>> {
    let mut buf = vec![0; size];
    let n = stream.peek(&mut buf).await?;

    if n == size {
        Ok(buf)
    } else {
        bail!("Peek size mismatch: {} != {}", n, size)
    }
}

async fn splice(mut inbound: TcpStream, mut outbound: TcpStream) -> Fallible<()> {
    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    // TODO: use splice(2) syscall
    let client_to_server = tokio::io::copy(&mut ri, &mut wo);
    let server_to_client = tokio::io::copy(&mut ro, &mut wi);

    future::try_join(client_to_server, server_to_client).await?;

    Ok(())
}

async fn process(mut inbound: TcpStream) -> Fallible<()> {
    let buf = peek(&mut inbound, TLS_RECORD_HEADER_LENGTH).await?;
    let mut rd = Reader::init(&buf);

    let content_type = ContentType::read(&mut rd).unwrap();
    let protocol_version = ProtocolVersion::read(&mut rd).unwrap();
    let handshake_size = usize::from(u16::read(&mut rd).unwrap());

    log!(
        Level::Debug,
        "Content type: {:?}, protocol version: {:?}, handshake size: {}",
        content_type,
        protocol_version,
        handshake_size
    );

    if content_type != ContentType::Handshake {
        bail!("TLS message is not a handshake");
    }

    if handshake_size > TLS_HANDSHAKE_MAX_LENGTH {
        bail!(
            "TLS handshake size is {} > {}",
            handshake_size,
            TLS_HANDSHAKE_MAX_LENGTH
        );
    }

    let buf = peek(&mut inbound, TLS_RECORD_HEADER_LENGTH + handshake_size).await?;
    let mut rd = Reader::init(&buf);
    rd.take(TLS_RECORD_HEADER_LENGTH);

    let handshake = HandshakeMessagePayload::read_version(&mut rd, protocol_version).unwrap();

    let client_hello = match handshake.payload {
        HandshakePayload::ClientHello(x) => x,
        _ => bail!("TLS handshake is not Client Hello"),
    };

    let sni = match client_hello.get_sni_extension() {
        Some(x) => x,
        None => bail!("Missing SNI"),
    };

    let host: &str = match &sni[0].payload {
        ServerNamePayload::HostName(x) => x.as_ref().into(),
        ServerNamePayload::Unknown(_) => bail!("Unknown SNI payload type"),
    };

    log!(Level::Debug, "SNI hostname: {}", host);

    if !host.ends_with("holohost.net") {
        bail!("Rejected {}", host);
    }

    let outbound_addr = format!("{}:443", host);
    let outbound = TcpStream::connect(outbound_addr).await?;
    splice(inbound, outbound).await
}

thread_local!(static UUID: RefCell<Uuid> = RefCell::new(Uuid::nil()));

#[tokio::main]
async fn main() -> Fallible<()> {
    env_logger::Builder::from_default_env()
        .format(|buf, record| {
            UUID.with(|f| {
                writeln!(
                    buf,
                    "[{} {} {:<5} {}] {}",
                    buf.timestamp(),
                    *f.borrow(),
                    buf.default_styled_level(record.level()),
                    record.target(),
                    record.args()
                )
            })
        })
        .init();

    let mut listener = TcpListener::bind("[::]:443").await?;

    loop {
        let (inbound, inbound_addr) = listener.accept().await?;

        tokio::spawn(async move {
            if log_enabled!(Level::Warn) {
                UUID.with(|f| {
                    *f.borrow_mut() = Uuid::new_v4();
                });
            }

            log!(
                Level::Info,
                "Accepted connection from {}",
                inbound_addr.ip()
            );

            if let Err(e) = process(inbound).await {
                log!(Level::Warn, "{}", e);
            }
        });
    }
}

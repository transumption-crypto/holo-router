// See: https://tls.ulfheim.net
use rustls::internal::msgs::codec::{Codec, Reader};
use rustls::internal::msgs::enums::{ContentType, ProtocolVersion};
use rustls::internal::msgs::handshake::{HandshakeMessagePayload, HandshakePayload, ServerNamePayload};

use std::env::args;
use std::net::{SocketAddr, ToSocketAddrs};
use std::error::Error;

use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

const TLS_RECORD_HEADER_LENGTH: usize = 5;

async fn peek(stream: &mut TcpStream, size: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut buf = vec![0; size];
    let n = stream.peek(&mut buf).await?;

    if n == size {
        Ok(buf)
    } else {
        Err(format!("size mismatch: {} != {}", n, size).into())
    }
}

async fn splice(inbound: TcpStream, outbound: TcpStream) -> Result<(), Box<dyn Error>> {
    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    // TODO: use splice(2) syscall
    let client_to_server = ri.copy(&mut wo);
    let server_to_client = ro.copy(&mut wi);

    Ok(())
}

fn as_addr<T: AsRef<str>>(host: T, port: u16) -> Option<SocketAddr> {
    match format!("{}:{}", host.as_ref(), port).to_socket_addrs() {
        Ok(mut addrs) => addrs.next(),
        Err(_) => None
    }
}

async fn process(mut inbound: TcpStream) -> Result<(), Box<dyn Error>> {
    let buf = peek(&mut inbound, TLS_RECORD_HEADER_LENGTH).await?;
    let mut rd = Reader::init(&buf);
    
    let content_type = ContentType::read(&mut rd).unwrap();
    let protocol_version = ProtocolVersion::read(&mut rd).unwrap();
    let handshake_size = usize::from(u16::read(&mut rd).unwrap());
    
    if content_type != ContentType::Handshake {
        return Err("TLS content type is not handshake".into());
    }
    
    let buf = peek(&mut inbound, TLS_RECORD_HEADER_LENGTH + handshake_size).await?;
    let mut rd = Reader::init(&buf);
    rd.take(TLS_RECORD_HEADER_LENGTH);

    let handshake = HandshakeMessagePayload::read_version(&mut rd, protocol_version).unwrap();

    let client_hello = match handshake.payload {
        HandshakePayload::ClientHello(x) => x,
        _ => {
            return Err("TLS handshake is not Client Hello".into());
        }
    };
    
    let sni = client_hello.get_sni_extension().unwrap();
    let host = match &sni[0].payload {
        ServerNamePayload::HostName(x) => x,
        ServerNamePayload::Unknown(_) => {
            return Err("Unknown SNI payload".into());
        }
    };
    
    let outbound = TcpStream::connect(&as_addr(host, 443).unwrap()).await?;
    splice(inbound, outbound).await
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = "0.0.0.0:443".parse()?;
    let mut listener = TcpListener::bind(&addr)?;

    loop {
        let (inbound, _) = listener.accept().await?;
        tokio::spawn(async move {
            if let Err(e) = process(inbound).await {
                println!("error: {:?}", e);
            }
        });
    }
}

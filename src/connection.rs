use rand::{distributions::Uniform, prelude::Distribution};
use std::{net::SocketAddr, thread, time::Duration};
use tokio::net::{TcpListener, UdpSocket};

#[derive(Clone, Debug)]
pub enum Protocol {
    Tcp,
    Udp,
}

pub enum Inbound {
    Tcp(TcpListener),
    Udp(UdpSocket),
}

pub async fn get_connection(address: SocketAddr, protocol: &Protocol) -> Inbound {
    loop {
        match protocol {
            Protocol::Tcp => match TcpListener::bind(address).await {
                Ok(listener) => return Inbound::Tcp(listener),
                Err(e) => sleep_log(address, e),
            },
            Protocol::Udp => match UdpSocket::bind(address).await {
                Ok(socket) => return Inbound::Udp(socket),
                Err(e) => sleep_log(address, e),
            },
        }
    }
}

fn sleep_log(address: SocketAddr, error: std::io::Error) {
    eprintln!(
        "Can't assign to random private address ({:?}): {:?}",
        address, error
    );
    thread::sleep(Duration::from_millis(500));
}

pub fn random_private_ip() -> [u8; 4] {
    let mut rng = rand::thread_rng();
    let address = Uniform::from(0..255);
    [
        127,
        address.sample(&mut rng),
        address.sample(&mut rng),
        address.sample(&mut rng),
    ]
}

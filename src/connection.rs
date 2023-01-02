use rand::{distributions::Uniform, prelude::Distribution};
use std::{
    net::SocketAddr, thread, time::Duration,
};
use tokio::{net::{TcpListener, UdpSocket}};

pub enum Protocol {
    Tcp,
    Udp,
}

pub enum Listener {
    Tcp(TcpListener),
    Udp(UdpSocket),
}

pub async fn get_connection(address: SocketAddr, protocol: &Protocol) -> Listener {
    loop {
        match protocol {
            Protocol::Tcp => match TcpListener::bind(address).await {
                Ok(listener) => return Listener::Tcp(listener),
                Err(e) => sleep_and_retry(address, e),
            },
            Protocol::Udp => match UdpSocket::bind(address).await {
                Ok(socket) => return Listener::Udp(socket),
                Err(e) => sleep_and_retry(address, e),
            },
        }
    }
}

fn sleep_and_retry(address: SocketAddr, error: std::io::Error) {
    eprintln!(
        "Failed assigned to random private address ({:?}): {:?}",
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

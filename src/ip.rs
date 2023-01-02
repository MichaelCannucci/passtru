use rand::{distributions::Uniform, prelude::Distribution};
use tokio::{net::TcpListener};
use std::{
    net::SocketAddr,
    thread,
    time::Duration,
};

pub async fn get_listener(port: u16) -> TcpListener {
    loop {
        let ip = random_private_ip();
        let address = SocketAddr::from((ip, port));
        match TcpListener::bind(address).await {
            Ok(l) => return l,
            Err(e) => {
                eprintln!(
                    "Failed assigned to random private address ({:?}): {:?}",
                    address, e
                );
                thread::sleep(Duration::from_millis(500));
            }
        }
    }
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
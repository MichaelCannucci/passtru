use std::{collections::HashMap, net::SocketAddr};

use tokio::{net::TcpStream, task::JoinHandle};

use crate::{
    connection::{get_connection, random_private_ip, Listener},
    info::{ProxyPort, ProxyableContainer},
};

type ProxyJoinHandle = JoinHandle<()>;

pub struct ProxyManager {
    proxies: HashMap<String, Vec<ProxyJoinHandle>>,
}

impl ProxyManager {
    pub fn new() -> Self {
        ProxyManager {
            proxies: Default::default(),
        }
    }

    pub async fn container_created(
        &mut self,
        container: ProxyableContainer,
    ) -> Result<(), std::io::Error> {
        if self.proxies.contains_key(&container.id) {
            return Ok(());
        }

        let mut proxies = Vec::<ProxyJoinHandle>::new();
        for port in container.ports {
            let proxy = start_proxy(&container.id, port).await?;
            proxies.push(proxy);
        }

        self.proxies.insert(container.id, proxies);

        Ok(())
    }

    pub fn container_removed(&mut self, container_id: &String) {
        if let Some(proxies) = self.proxies.remove(container_id) {
            println!("Stopping proxy for container ({0})", container_id);
            for proxy in proxies {
                proxy.abort();
            }
        }
    }
}

async fn start_proxy<'a>(
    container_id: &String,
    container: ProxyPort,
) -> Result<ProxyJoinHandle, std::io::Error> {
    let ip = random_private_ip();
    let address = SocketAddr::from((ip, container.private_port));

    println!(
        "Creating proxy for container ({0}) \n Using address: {1} for 127.0.0.1:{2}",
        container_id, address, container.public_port
    );

    Ok(tokio::spawn(async move {
        let incoming_connection = get_connection(address, &container.protocol).await;
        match incoming_connection {
            Listener::Tcp(tcp_listener) => {
                loop {
                    let (incoming, _) = tcp_listener.accept().await.unwrap();
                    let destination = TcpStream::connect(("127.0.0.1", container.public_port))
                        .await
                        .unwrap();
    
                    if let Some(err) = proxy_tcp_request(incoming, destination).await {
                        eprintln!("{:?}", err);
                    }
                }
            }
            Listener::Udp(_) => todo!(),
        }
    }))
}

async fn proxy_tcp_request(
    mut incoming: TcpStream,
    mut destination: TcpStream,
) -> Option<tokio::io::Error> {
    let (mut incoming_recv, mut incoming_send) = incoming.split();
    let (mut destination_recv, mut destination_send) = destination.split();

    let handle_one = async { tokio::io::copy(&mut incoming_recv, &mut destination_send).await };

    let handle_two = async { tokio::io::copy(&mut destination_recv, &mut incoming_send).await };

    match tokio::try_join!(handle_one, handle_two) {
        Ok(_) => None,
        Err(err) => Some(err),
    }
}

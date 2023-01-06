use std::{collections::HashMap, net::SocketAddr};

use tokio::{
    io::AsyncWriteExt,
    net::{TcpListener, TcpStream, UdpSocket},
    task::JoinHandle,
};

use crate::{
    connection::{get_connection, random_private_ip, Inbound},
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

        // Note: for hostname mapping in the hosts file, follow these rules in regards to the public/private ports
        // if they are the same, add original address that docker was bound to (i.e., 0.0.0.0, localhost)
        // if they are different, add address that was bound (There can be both at the same time)

        for port in container.ports {
            if port.private_port == port.public_port {
                continue;
            }

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
    port: ProxyPort,
) -> Result<ProxyJoinHandle, std::io::Error> {
    let ip = random_private_ip().clone();
    let address = SocketAddr::from((ip, port.private_port));

    let id = container_id.clone();

    Ok(tokio::spawn(async move {
        println!(
            "Creating proxy for container ({0}) \n Using address: {1} for 127.0.0.1:{2}",
            id, address, port.public_port
        );

        let listener_connection = get_connection(address, &port.protocol).await;
        loop {
            let result = match &listener_connection {
                Inbound::Tcp(tcp_listener) => {
                    proxy_tcp(tcp_listener, port.public_port).await
                }
                Inbound::Udp(socket) => {
                    proxy_udp(socket, port.public_port).await
                }
            };

            if let Some(err) = result {
                eprintln!("{:#?}", err)
            }
        }
    }))
}

async fn proxy_udp(inbound: &UdpSocket, public_port: u16) -> Option<tokio::io::Error> {
    let outbound = UdpSocket::bind(("127.0.0.1", public_port)).await.ok()?;

    let join = tokio::try_join!(
        async {
            let mut buf = vec![0; 1024];
            while let Ok((size, peer)) = inbound.recv_from(&mut buf).await {
                let amt = outbound.send_to(&buf[..size], &peer).await?;
                println!("Echoed {}/{} bytes to {}", amt, size, peer);
            }
            Ok(())
        },
        async {
            let mut buf = vec![0; 1024];
            while let Ok((size, peer)) = outbound.recv_from(&mut buf).await {
                let amt = inbound.send_to(&buf[..size], &peer).await?;
                println!("Echoed {}/{} bytes to {}", amt, size, peer);
            }
    
            Ok(())
        }
    );
        
    match join {
        Err(err) => Some(err),
        Ok(_) => None,
    }

}

async fn proxy_tcp(listener: &TcpListener, public_port: u16) -> Option<tokio::io::Error> {
    loop {
        let (mut inbound, _) = listener.accept().await.unwrap();
        let mut destination = TcpStream::connect(("127.0.0.1", public_port))
            .await
            .unwrap();

        let (mut inbound_rv, mut inbound_tx) = inbound.split();
        let (mut outbound_rv, mut outbound_tx) = destination.split();

        let join = tokio::try_join!(
            async {
                let res = tokio::io::copy(&mut inbound_rv, &mut outbound_tx).await;
                outbound_tx.shutdown().await?;

                res
            },
            async {
                let res = tokio::io::copy(&mut outbound_rv, &mut inbound_tx).await;
                inbound_tx.shutdown().await?;

                res
            }
        );

        if let Err(err) = join {
            return Some(err);
        }
    }
}

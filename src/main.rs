use std::net::SocketAddr;

use futures::{future::join_all, StreamExt};
use info::ProxyableContainerInformation;
use shiplift::Docker;
use tokio::{net::TcpStream, task::JoinHandle};

use crate::{
    connection::{get_connection, random_private_ip, Listener},
    info::filter_proxyable_containers,
};

mod connection;
mod info;

type ProxyJoinHandle = JoinHandle<()>;

#[tokio::main]
async fn main() {
    let docker = Docker::new();
    println!("listening for events");

    let containers = docker.containers().list(&Default::default()).await.unwrap();

    let process_proxy = async {
        println!("Starting passthrough");
        let mut proxies: Vec<ProxyJoinHandle> = Vec::new();
        for info in filter_proxyable_containers(containers) {
            let proxy = match start_proxy(info).await {
                Err(err) => {
                    eprintln!("Error starting thread {:?}", err);
                    continue;
                }
                Ok(proxy) => proxy,
            };
            proxies.push(proxy);
        }

        join_all(proxies).await;
    };

    let process_docker_events = async {
        println!("Listening for docker events");
        while let Some(event_result) = docker.events(&Default::default()).next().await {
            match event_result {
                Ok(event) => println!("{:?}", event),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    };

    tokio::join!(process_proxy, process_docker_events);
}

async fn start_proxy(
    proxy: ProxyableContainerInformation,
) -> Result<ProxyJoinHandle, std::io::Error> {
    let ip = random_private_ip();
    let address = SocketAddr::from((ip, proxy.private_port));

    println!(
        "Creating proxy for container ({0}) \n Using address: {1} for 127.0.0.1:{2}",
        proxy.id, address, proxy.public_port
    );

    Ok(tokio::spawn(async move {
        // Todo: Avoid recreating connection each time, maybe try using Arc?
        loop {
            let incoming_connection = get_connection(address, &proxy.protocol).await;
            match incoming_connection {
                Listener::Tcp(tcp_listener) => {
                    let (incoming, _) = tcp_listener.accept().await.unwrap();
                    let destination = TcpStream::connect(("127.0.0.1", proxy.public_port))
                        .await
                        .unwrap();
                        
                    match proxy_tcp_request(incoming, destination).await {
                        Some(err) => eprintln!("{:?}", err),
                        None => println!("finished proxing the request"),
                    }
                }
                Listener::Udp(_) => todo!(),
            }
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

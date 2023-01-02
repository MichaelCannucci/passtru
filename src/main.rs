use futures::{StreamExt, future::join_all};
use shiplift::Docker;
use tokio::{net::TcpStream, task::JoinHandle};

use crate::{ip::get_listener, info::container_information};

mod ip;
mod info;

type ProxyJoinHandle = JoinHandle<()>;

#[tokio::main]
async fn main() {
    let docker = Docker::new();
    println!("listening for events");

    let containers = docker.containers().list(&Default::default()).await.unwrap();

    let process_proxy = async {
        let mut proxies: Vec<ProxyJoinHandle> = Vec::new();
        for info in container_information(containers) {
            let proxy = match start_proxy(info.private_port, info.public_port).await {
                Err(err) => {
                    eprintln!("Error starting thread {:?}", err);
                    continue;
                },
                Ok(proxy) => proxy
            };
            proxies.push(proxy);
        }

        join_all(proxies).await;
    };

    let process_docker_events = async {
        while let Some(event_result) = docker.events(&Default::default()).next().await {
            match event_result {
                Ok(event) => println!("{:?}", event),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    };

    tokio::join!(process_proxy, process_docker_events);
}

async fn start_proxy(private_port: u16, public_port: u16) -> Result<ProxyJoinHandle, std::io::Error> {
    let listener = get_listener(private_port).await;

    println!(
        "Got address for listener: {:?}:{:?}",
        listener.local_addr()?.ip(),
        listener.local_addr()?.port()
    );

    println!("Starting passthrough");

    Ok(tokio::spawn(async move { 
        loop {
            let (incoming, _) = listener.accept().await.unwrap();
            let destination = TcpStream::connect(("127.0.0.1", public_port)).await.unwrap();

            match proxy_request(incoming, destination).await {
                Some(err) => eprintln!("{:?}", err),
                None => println!("finished proxing the request")
            }   
        }
    }))
}

async fn proxy_request(
    mut incoming: TcpStream,
    mut destination: TcpStream,
) -> Option<tokio::io::Error> {
    let (mut incoming_recv, mut incoming_send) = incoming.split();
    let (mut destination_recv, mut destination_send) = destination.split();

    
    let handle_one = async {
        println!("Proxing to container");
        tokio::io::copy(&mut incoming_recv, &mut destination_send).await
    };

    let handle_two = async {
        println!("Proxing response from container");
        tokio::io::copy(&mut destination_recv, &mut incoming_send).await
    };

    match tokio::try_join!(handle_one, handle_two) {
        Ok(_) => None,
        Err(err) => Some(err)
    }
}
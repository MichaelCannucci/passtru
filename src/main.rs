use crate::info::{get_proxyable_containers, get_proxyable_information};
use futures::StreamExt;
use shiplift::Docker;

use proxy::ProxyManager;

mod connection;
mod info;
mod proxy;

#[tokio::main]
async fn main() {
    let docker = Docker::new();
    let mut manager = ProxyManager::new();

    let containers = docker.containers().list(&Default::default()).await.unwrap();
    for info in get_proxyable_containers(containers) {
        if let Err(err) = manager.container_created(info).await {
            eprintln!("{:#?}", err);
        }
    }

    println!("Listening for docker events");
    while let Some(event_result) = docker.events(&Default::default()).next().await {
        if let Ok(event) = event_result {
            let event_container = docker.containers().get(&event.actor.id);
            match event.action.as_str() {
                "start" => {
                    if let Ok(details) = event_container.inspect().await {
                        let info = get_proxyable_information(details);
                        if let Err(err) = manager.container_created(info).await {
                            eprintln!("{:#?}", err);
                        }
                    }
                }
                "kill" | "destroy" | "die" => manager.container_removed(&event.actor.id),
                _ => println!("{0}", event.action),
            };
        }
    }
}

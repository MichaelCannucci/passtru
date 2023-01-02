// use futures::{StreamExt};
use itertools::Itertools;
use rand::Rng;
use shiplift::Docker;
use tokio::join;
use std::{
    net::{SocketAddr, TcpListener, TcpStream},
    thread, time::{self, Duration}, io,
};

#[tokio::main]
async fn main() {
    let docker = Docker::new();
    println!("listening for events");

    let containers = docker.containers().list(&Default::default()).await.unwrap();

    let mut threads: Vec<_> = containers
        .iter()
        .flat_map(|container| {
            container
                .ports
                .iter()
                .unique_by(|port| port.private_port)
                // Cloning because I can't figure out lifetimes :(
                .map(|port| (port.clone(), container.id.clone()))
        })
        .filter_map(|args| {
            let (port, id) = args;
            println!("Creating passthrough for {:?} for {:?}", id, port);
            let public_port = match port.public_port {
                Some(p) => p,
                None => {
                    println!("Skipping because there is no public port");
                    return None;
                }
            }.to_owned();

            Some(tokio::spawn(async move { passthrough( port.private_port as u16, public_port as u16) }))
        })
        .collect();

    while let Some(thread) = threads.pop() {
        println!("Waiting...");
        let result = join!(thread).0;
        match result {
            Ok(r) => {
                match r.await {
                    Err(err) => eprintln!("{:?}", err),
                    _ => println!("Thread finished")
                }
            },
            Err(err) => eprintln!("{:?}", err),
        }
    }

    // while let Some(event_result) = docker.events(&Default::default()).next().await {
    //     match event_result {
    //         Ok(event) => println!("{:?}", event),
    //         Err(e) => eprintln!("Error: {}", e),
    //     }
    // }
}

async fn passthrough(
    private_port: u16,
    public_port: u16,
) -> Result<(), io::Error> { 
    println!("Starting passthrough");

    let listener = get_listener(private_port);

    println!(
        "Got address for listener: {:?}:{:?}", 
        listener.local_addr()?.ip(), 
        listener.local_addr()?.port()
    );

    for src in listener.incoming() {
        // Read timeout is really bad, this needs to be refactored in the future
        let mut src = src?;
        src.set_read_timeout(Some(Duration::from_millis(1)))?;
    
        let mut dst = TcpStream::connect(("127.0.0.1", public_port))?;
        dst.set_read_timeout(Some(Duration::from_millis(1)))?;
        
        match io::copy(&mut src, &mut dst) {
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => (),
            Err(err) => return Err(err),
            Ok(_) => ()
        };
    
        match io::copy(&mut dst, &mut src) {
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => (),
            Err(err) => return Err(err),
            Ok(_) => ()
        };
    }

    Ok(())
}

fn get_listener(port: u16) -> TcpListener {
    loop {
        let ip = random_private_ip();
        let address = SocketAddr::from((ip, port));
        match TcpListener::bind(address) {
            Ok(l) => return l,
            Err(e) => {
                eprintln!(
                    "Failed assigned to random private address ({:?}): {:?}",
                    address, e
                );
                thread::sleep(time::Duration::from_millis(500));
            }
        }
    }
}

fn random_private_ip() -> [u8; 4] {
    let mut rng = rand::thread_rng();
    [
        127,
        rng.gen_range(0..255),
        rng.gen_range(0..255),
        rng.gen_range(0..255),
    ]
}

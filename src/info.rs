use shiplift::rep::Container;
use itertools::Itertools;

pub struct ProxyableContainerInformation 
{
    pub public_port: u16,
    pub private_port: u16,
    pub id: String
}

pub fn container_information(containers: Vec<Container>) -> Vec<ProxyableContainerInformation> {
    let ports: Vec<_> = containers
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
                    return None;
                }
            };
            Some(ProxyableContainerInformation {
                public_port: public_port as u16,
                private_port: port.private_port as u16,
                id
            })
        })
        .collect();

    ports
}

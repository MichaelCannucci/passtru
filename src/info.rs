use shiplift::rep::Container;
use itertools::Itertools;

use crate::connection::Protocol;

pub struct ProxyableContainerInformation 
{
    pub public_port: u16,
    pub private_port: u16,
    pub id: String,
    pub protocol: Protocol,
}

pub fn filter_proxyable_containers(containers: Vec<Container>) -> Vec<ProxyableContainerInformation> {
    let ports: Vec<_> = containers
        .iter()
        .flat_map(|container| {
            container
                .ports
                .iter()
                .unique_by(|port| port.private_port)
                .map(|port| (port, &container.id))
        })
        .filter_map(|args| {
            let (port, id) = args;
            let public_port = match port.public_port {
                Some(p) => p,
                None => {
                    return None;
                }
            };
            let protocol = match port.typ.as_str() {
                "tcp" => Protocol::Tcp,
                "udp" => Protocol::Udp,
                _ => return None
            };

            Some(ProxyableContainerInformation {
                public_port: public_port as u16,
                private_port: port.private_port as u16,
                protocol,
                id: id.clone()
            })
        })
        .collect();

    ports
}

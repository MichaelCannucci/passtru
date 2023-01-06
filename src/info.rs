use itertools::Itertools;
use shiplift::rep::{Container, ContainerDetails};

use crate::connection::Protocol;

#[derive(Clone, Debug)]
pub struct ProxyableContainer {
    pub ports: Vec<ProxyPort>,
    pub id: String,
}

#[derive(Clone, Debug)]
pub struct ProxyPort {
    pub public_port: u16,
    pub private_port: u16,
    pub protocol: Protocol,
}

pub fn get_proxyable_containers(containers: Vec<Container>) -> Vec<ProxyableContainer> {
    let mut results = Vec::<ProxyableContainer>::new();

    for container in containers {
        let ports = container
            .ports
            .iter()
            .unique_by(|port| port.private_port)
            .filter_map(|port| {
                let public_port = match port.public_port {
                    Some(p) => p,
                    None => {
                        return None;
                    }
                };

                let protocol = match port.typ.as_str() {
                    "tcp" => Protocol::Tcp,
                    "udp" => Protocol::Udp,
                    _ => return None,
                };

                Some(ProxyPort {
                    public_port: public_port as u16,
                    private_port: port.private_port as u16,
                    protocol: protocol,
                })
            })
            .collect();

        results.push(ProxyableContainer {
            id: container.id.clone(),
            ports,
        })
    }

    results
}

pub fn get_proxyable_information(details: ContainerDetails) -> ProxyableContainer {
    let ports: Vec<ProxyPort> = details
        .host_config
        .port_bindings
        .iter()
        .flatten()
        .filter_map(|args| {
            let (key, ports) = args;
            let (private_port_str, protocol) = key.split('/').collect_tuple()?;

            let private_port = private_port_str.parse::<u16>().ok()?;

            let protocol = match protocol {
                "tcp" => Protocol::Tcp,
                "udp" => Protocol::Udp,
                _ => return None,
            };

            let proxy_ports: Vec<ProxyPort> = ports
                .iter()
                .filter_map(|binding| {
                    let public_port = binding.get("HostPort")?.parse::<u16>().ok()?;

                    Some(ProxyPort {
                        private_port,
                        public_port,
                        protocol: protocol.clone(),
                    })
                })
                .collect();

            Some(proxy_ports)
        })
        .flatten()
        .collect();

    ProxyableContainer {
        id: details.id.clone(),
        ports,
    }
}

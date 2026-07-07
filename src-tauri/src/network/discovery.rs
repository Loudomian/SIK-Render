use crate::network::peer;
use crate::network::server::build_node_info;
use crate::network::types::SERVICE_TYPE;
use crate::state::AppState;
use mdns_sd::{IfKind, ServiceDaemon, ServiceEvent, ServiceInfo};
use std::net::{IpAddr, Ipv4Addr};
use tauri::AppHandle;

pub fn start(app: AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        let node = build_node_info(&app, &state).await;
        let node_id = node.id.clone();

        let mdns = match ServiceDaemon::new() {
            Ok(daemon) => daemon,
            Err(error) => {
                log::error!("mDNS daemon failed to start: {error}");
                return;
            }
        };

        configure_mdns_interface(&mdns, &node.ip_address);

        let properties = [("id", node.id.as_str()), ("version", node.version.as_str())];
        match ServiceInfo::new(
            SERVICE_TYPE,
            &node_id,
            &format!("{}.local.", node.hostname),
            node.ip_address.as_str(),
            node.port,
            &properties[..],
        ) {
            Ok(service) => {
                if let Err(error) = mdns.register(service) {
                    log::error!("mDNS register failed: {error}");
                }
            }
            Err(error) => log::error!("mDNS service creation failed: {error}"),
        }

        let receiver = match mdns.browse(SERVICE_TYPE) {
            Ok(receiver) => receiver,
            Err(error) => {
                log::error!("mDNS browse failed: {error}");
                return;
            }
        };

        loop {
            match receiver.recv_async().await {
                Ok(ServiceEvent::ServiceResolved(info)) => {
                    let instance = info.get_fullname();
                    if instance.contains(&node_id) {
                        continue;
                    }

                    let peer_id = extract_instance_name(instance);
                    let Some(peer_ip) = info.get_addresses().iter().find_map(|ip| match ip {
                        IpAddr::V4(ip) => Some(ip.to_string()),
                        IpAddr::V6(_) => None,
                    }) else {
                        log::warn!("Node discovered without address: {peer_id}");
                        continue;
                    };
                    let peer_port = info.get_port();

                    log::info!("Node discovered: {peer_id} @ {peer_ip}:{peer_port}");
                    peer::spawn_connection(app.clone(), state.clone(), peer_id, peer_ip, peer_port);
                }
                Ok(ServiceEvent::ServiceRemoved(_, fullname)) => {
                    let peer_id = extract_instance_name(&fullname);
                    if peer_id == node_id {
                        continue;
                    }
                    log::info!("Node lost: {peer_id}");
                    peer::remove(app.clone(), state.clone(), peer_id).await;
                }
                Ok(_) => {}
                Err(error) => {
                    log::error!("mDNS recv error: {error}");
                    break;
                }
            }
        }
    });
}

fn configure_mdns_interface(mdns: &ServiceDaemon, advertised_ip: &str) {
    let Ok(ip) = advertised_ip.parse::<Ipv4Addr>() else {
        let _ = mdns.disable_interface(IfKind::IPv6);
        log::warn!("mDNS could not parse advertised IPv4 address: {advertised_ip}");
        return;
    };

    if ip.is_loopback() {
        let _ = mdns.disable_interface(IfKind::IPv6);
        log::warn!("mDNS is using loopback; IPv6 interfaces are disabled for node discovery");
        return;
    }

    if let Err(error) = mdns.disable_interface(IfKind::All) {
        log::warn!("mDNS failed to disable all interfaces before scoping: {error}");
    }
    if let Err(error) = mdns.enable_interface(IfKind::Addr(IpAddr::V4(ip))) {
        log::warn!("mDNS failed to enable interface {ip}: {error}");
        if let Err(fallback_error) = mdns.enable_interface(IfKind::All) {
            log::warn!("mDNS failed to restore all interfaces after scoping failure: {fallback_error}");
        } else {
            let _ = mdns.disable_interface(IfKind::IPv6);
            log::warn!("mDNS fell back to all IPv4 interfaces after scoping failure");
        }
    } else {
        log::info!("mDNS scoped to IPv4 interface {ip}");
    }
}

fn extract_instance_name(fullname: &str) -> String {
    fullname.split('.').next().unwrap_or(fullname).to_string()
}

mod node;
mod network;
mod validation;
mod consensus;
mod storage;
mod config;
mod init;

use actix_web::{ App, HttpServer, web };
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::Duration;
use std::net::SocketAddr;
use crate::node::node::{ NodeList, Node };
use crate::config::Config;
use crate::init::{
    NodeInfo,
    resolve_address,
    fetch_and_update_nodes,
    register_with_discovery_service,
};
use crate::storage::Storage;
use tracing::info;

const NODE_INFO_FILE: &str = "node_info.json";
const CONFIG_FILE: &str = "config.json";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();

    info!("Starting application...");

    // Load the configuration (UUID and address)
    let config = Config::load(CONFIG_FILE).expect("Failed to create or fetch UUID and address");

    // If the address is not an IP:Port, resolve it using the resolve_address function.
    let server_address = if let Ok(socket_addr) = config.address.parse::<SocketAddr>() {
        socket_addr
    } else {
        // Attempt to resolve the address
        if let Err(e) = resolve_address(&config.address).await {
            eprintln!("Failed to resolve node address: {}", e);
            return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
        }
        // After resolution, fallback to a default socket address if needed
        "127.0.0.1:8080".parse().expect("Failed to parse fallback address")
    };

    // Fetch and update nodes from the discovery service
    let node_info = fetch_and_update_nodes(NODE_INFO_FILE).await.unwrap_or_else(|_| NodeInfo {
        nodes: vec![],
    });

    let node_list = Arc::new(Mutex::new(NodeList::from_nodes(node_info.nodes)));

    let node = {
        let node_list_guard = node_list.lock().await;
        if let Some(existing_node) = node_list_guard.find_node_by_uuid(&config.uuid) {
            existing_node.clone()
        } else {
            let new_node = Node::new(&config.address);
            register_with_discovery_service(
                &new_node,
                config.uuid.clone(),
                config.address.clone()
            ).await.unwrap();
            node_list_guard.add_node(new_node.clone());
            new_node
        }
    };

    info!("Node ID: {}", node.id);
    info!("Public Key: {}", node.public_key);

    let storage = Arc::new(Mutex::new(Storage::new("database/db")));

    let node_list_clone = Arc::clone(&node_list);
    tokio::spawn(async move {
        loop {
            match fetch_and_update_nodes(NODE_INFO_FILE).await {
                Ok(updated_node_info) => {
                    let updated_node_list = NodeList::from_nodes(updated_node_info.nodes);
                    let mut node_list_guard = node_list_clone.lock().await;
                    *node_list_guard = updated_node_list;
                    info!("Node list updated.");
                }
                Err(e) => tracing::error!("Failed to update node list: {}", e),
            }

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    // Bind and run the server using the resolved or fallback address
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Arc::clone(&node_list)))
            .app_data(web::Data::new(Arc::clone(&storage)))
            .configure(network::api::init_routes)
    })
        .bind(server_address)?
        .run().await
}

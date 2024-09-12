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
use reqwest::Client;
use anyhow::Result;
use serde_json::json;

const NODE_INFO_FILE: &str = "node_info.json";
const CONFIG_FILE: &str = "config.json";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();

    info!("Starting application...");

    // Load the configuration (UUID and address)
    let mut config = Config::load(CONFIG_FILE).expect("Failed to create or fetch UUID and address");

    // Check if the wallet address is missing
    if config.wallet_address.is_none() {
        // Prompt the user for a wallet address
        match Config::prompt_for_wallet_address() {
            Ok(wallet_address) => {
                // Save the wallet address in the config
                config.wallet_address = Some(wallet_address);
                config.save(CONFIG_FILE).expect("Failed to save the config with wallet address.");
            }
            Err(e) => {
                eprintln!("Error getting wallet address: {}", e);
                return Err(
                    std::io::Error::new(std::io::ErrorKind::Other, "Failed to get wallet address")
                );
            }
        }
    }

    // Proceed with the rest of the logic using the wallet address
    let wallet_address = config.wallet_address.as_ref().unwrap();

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
    let client = Client::new(); // Create a reqwest client for making HTTP requests

    // Task to update node list periodically
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

    // Task to check node availability and remove unreachable nodes after 3 failed cycles
    let node_list_clone_for_check = Arc::clone(&node_list);
    tokio::spawn(async move {
        if let Err(e) = check_and_remove_unavailable_nodes(node_list_clone_for_check, client).await {
            tracing::error!("Error checking and removing nodes: {}", e);
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

// Check if nodes are available and remove them after 3 failed checks
async fn check_and_remove_unavailable_nodes(
    node_list: Arc<Mutex<NodeList>>,
    client: Client
) -> Result<()> {
    let mut failed_attempts: Vec<(String, u8)> = Vec::new(); // Track failed attempts (UUID, failed count)

    loop {
        // Lock the node list to check availability
        let nodes = {
            let node_list_guard = node_list.lock().await;
            node_list_guard.get_nodes().clone()
        };

        // Check each node's availability
        for node in nodes {
            let node_id = node.id.clone();
            let node_available = check_node_availability(&node).await;

            // If the node is unavailable
            if !node_available {
                // Find if the node is already being tracked for failed attempts
                if let Some(entry) = failed_attempts.iter_mut().find(|(id, _)| id == &node_id) {
                    entry.1 += 1; // Increment the failure count
                } else {
                    failed_attempts.push((node_id.clone(), 1)); // Start tracking this node
                }

                // If the node has failed 3 times, remove it
                if
                    let Some((_, _count)) = failed_attempts
                        .iter()
                        .find(|(id, count)| id == &node_id && *count >= 3)
                {
                    // Remove node and send delete request
                    remove_node(&node_id, &node_list, &client).await?;
                    info!("Node {} removed after 3 failed attempts", node_id);
                }
            } else {
                // If node is available, reset the failure count
                failed_attempts.retain(|(id, _)| id != &node_id);
            }
        }

        // Sleep for 5 seconds before the next cycle
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

// Check if the node is available by pinging or validating its availability (mock logic)
async fn check_node_availability(node: &Node) -> bool {
    // Replace this with the actual logic to check if a node is available
    // For example, you can ping the node, send a request, etc.
    println!("Checking availability for node: {}", node.id);
    true // Mocking that the node is always available for now
}

// Remove the node from the NodeList and call the delete_node endpoint
async fn remove_node(
    node_id: &str,
    node_list: &Arc<Mutex<NodeList>>,
    client: &Client
) -> Result<()> {
    // Remove the node from the NodeList
    let node_list_guard = node_list.lock().await; // Removed `mut` here
    if node_list_guard.remove_node_by_uuid(node_id) {
        info!("Node {} successfully removed from local node list", node_id);
    } else {
        eprintln!("Node {} not found in local node list", node_id);
        return Err(anyhow::anyhow!("Node {} not found", node_id));
    }

    // Call the delete_node API
    let delete_node_body = json!({ "id": node_id });
    let response = client
        .post("https://synnq-discovery-f77aaphiwa-uc.a.run.app/delete_node")
        .json(&delete_node_body)
        .send().await?;

    if response.status().is_success() {
        info!("Successfully removed node {} from the discovery service", node_id);
    } else {
        eprintln!(
            "Failed to remove node {} from the discovery service. Status: {}",
            node_id,
            response.status()
        );
        return Err(
            anyhow::anyhow!("Failed to remove node {}. Status: {}", node_id, response.status())
        );
    }

    Ok(())
}

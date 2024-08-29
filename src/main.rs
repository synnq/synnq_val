mod node;
mod api;
mod validation;
mod consensus;
mod storage;
mod config;

use actix_web::{ App, HttpServer };
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use std::fs;
use std::sync::{ Arc, Mutex };
use std::error::Error;
use std::io::{ self, Write };
use uuid::Uuid;
use tokio::time::{ sleep, Duration };
use crate::node::{ NodeList, Node };
use crate::config::Config;

const CONFIG_FILE: &str = "config.json";
const NODE_INFO_FILE: &str = "node_info.json";
const DISCOVERY_SERVICE_URL: &str = "https://synnq-discovery-f77aaphiwa-uc.a.run.app";

#[derive(Serialize)]
struct RegisterNodeRequest {
    id: String,
    address: String,
    public_key: String,
}

#[derive(Serialize, Deserialize)]
struct NodeInfo {
    nodes: Vec<Node>,
}

// Check if config.json exists, and create if not. Check or create UUID.
fn check_or_create_uuid() -> io::Result<String> {
    // Check if the config file exists and read its contents
    if let Ok(config_data) = fs::read_to_string(CONFIG_FILE) {
        // Try to deserialize the config file to get the UUID
        if let Ok(config) = serde_json::from_str::<Config>(&config_data) {
            return Ok(config.uuid);
        }
    }

    // If the file does not exist or deserialization failed, create a new UUID
    let new_uuid = Uuid::new_v4().to_string();

    // Create a new config object with the UUID
    let new_config = Config { uuid: new_uuid.clone() };

    // Serialize the new config and save it to the config file
    let config_json = serde_json::to_string_pretty(&new_config)?;
    fs::write(CONFIG_FILE, config_json)?;

    Ok(new_uuid)
}

// Fetch and update nodes from discovery service
async fn fetch_and_update_nodes(node_info_file: &str) -> Result<NodeInfo, Box<dyn Error>> {
    let client = Client::new();
    let discovery_service_url = format!("{}/nodes", DISCOVERY_SERVICE_URL);

    // Send a GET request to the discovery service
    let response = client.get(&discovery_service_url).send().await?;
    println!("Response: {:?}", response);

    // Check if the response is successful
    if response.status().is_success() {
        // Parse the response JSON into a Vec<Node>
        let nodes: Vec<Node> = response.json().await?;
        let node_info = NodeInfo { nodes };

        // Serialize the NodeInfo struct to a pretty JSON string
        let data = serde_json::to_string_pretty(&node_info)?;

        // Write the JSON string to the specified file
        fs::write(node_info_file, data)?;

        println!("Node information updated successfully.");

        // Return the node information
        Ok(node_info)
    } else {
        // Log an error message and return an error if the request failed
        eprintln!("Failed to fetch nodes from discovery service. Status: {}", response.status());
        Err(Box::new(response.error_for_status().unwrap_err()))
    }
}

// Register with the discovery service
async fn register_with_discovery_service(node: &Node, uuid: String) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let discovery_service_url = format!("{}/register_node", DISCOVERY_SERVICE_URL);

    let request_body = RegisterNodeRequest {
        id: uuid,
        address: node.address.clone(),
        public_key: node.public_key.clone(),
    };

    let response = client.post(&discovery_service_url).json(&request_body).send().await?;

    if response.status().is_success() {
        println!("Successfully registered with discovery service.");
    } else {
        eprintln!("Failed to register with discovery service. Status: {}", response.status());
    }

    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Step 1 & 2: Check or create config.json and UUID
    let uuid = check_or_create_uuid().expect("Failed to create or fetch UUID");

    // Step 3: Fetch and update nodes from discovery service
    let node_info = fetch_and_update_nodes(NODE_INFO_FILE).await.unwrap_or_else(|_| NodeInfo {
        nodes: vec![],
    });

    // Step 4: Initialize NodeList and determine if the current node needs to register
    let node_list = Arc::new(Mutex::new(NodeList::from_nodes(node_info.nodes)));

    let node = {
        let mut node_list_guard = node_list.lock().unwrap();
        if let Some(existing_node) = node_list_guard.find_node_by_uuid(&uuid) {
            existing_node.clone()
        } else {
            // Node is not in the list, so we create and register it
            let new_node = Node::new("127.0.0.1:8080"); // Or determine external IP
            register_with_discovery_service(&new_node, uuid.clone()).await.unwrap();
            node_list_guard.add_node(new_node.clone());
            new_node
        }
    };

    println!("Node ID: {}", node.id);
    println!("Public Key: {}", node.public_key);

    let storage = storage::Storage::new("database/db");

    // Step 6: Periodically fetch and update nodes every 5 seconds
    let node_list_clone = Arc::clone(&node_list);
    tokio::spawn(async move {
        loop {
            match fetch_and_update_nodes(NODE_INFO_FILE).await {
                Ok(updated_node_info) => {
                    let updated_node_list = NodeList::from_nodes(updated_node_info.nodes);
                    let mut node_list_guard = node_list_clone.lock().unwrap();
                    *node_list_guard = updated_node_list;
                    println!("Node list updated.");
                }
                Err(e) => eprintln!("Failed to update node list: {}", e),
            }

            sleep(Duration::from_secs(5)).await;
        }
    });

    HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new(Arc::clone(&node_list)))
            .app_data(actix_web::web::Data::new(storage.clone()))
            .configure(api::init_routes)
    })
        .bind(("127.0.0.1", 8080))?
        .run().await
}

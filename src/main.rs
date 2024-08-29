mod node;
mod api;
mod validation;
mod consensus;
mod storage;
mod config;

use actix_web::{ App, HttpServer, web };
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use std::fs;
use std::sync::{ Arc };
use std::error::Error;
use std::io::{ self, Write };
use uuid::Uuid;
use tokio::time::{ sleep, Duration };
use std::net::SocketAddr;
use crate::node::{ NodeList, Node };
use crate::config::Config;
use crate::storage::Storage;
use tracing::{ debug, info };
use tracing_subscriber::fmt::Subscriber;

const CONFIG_FILE: &str = "config.json";
const NODE_INFO_FILE: &str = "node_info.json";
const DISCOVERY_SERVICE_URL: &str = "https://synnq-discovery-f77aaphiwa-uc.a.run.app";
// const DISCOVERY_SERVICE_URL: &str = "http://127.0.0.1:8080";

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
fn check_or_create_uuid() -> io::Result<Config> {
    // Check if the config file exists and read its contents
    if let Ok(config_data) = fs::read_to_string(CONFIG_FILE) {
        // Try to deserialize the config file to get the UUID and address
        if let Ok(config) = serde_json::from_str::<Config>(&config_data) {
            return Ok(config);
        }
    }

    // If the file does not exist or deserialization failed, create a new UUID and address
    let new_uuid = Uuid::new_v4().to_string();

    // Prompt the user to input the node's address
    let mut input_address = String::new();
    println!("Enter the node's address (e.g., 127.0.0.1:8080): ");
    io::stdin().read_line(&mut input_address)?;
    let input_address = input_address.trim().to_string();

    // Create a new config object with the UUID and address
    let new_config = Config {
        uuid: new_uuid.clone(),
        address: input_address.clone(),
    };

    // Serialize the new config and save it to the config file
    let config_json = serde_json::to_string_pretty(&new_config)?;
    fs::write(CONFIG_FILE, config_json)?;

    Ok(new_config)
}

// Fetch and update nodes from the discovery service
async fn fetch_and_update_nodes(
    node_info_file: &str
) -> Result<NodeInfo, Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let discovery_service_url = format!("{}/nodes", DISCOVERY_SERVICE_URL);

    // Send a GET request to the discovery service
    let response = client.get(&discovery_service_url).send().await?;

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
        let error_text = response.text().await?;
        Err(format!("API error body: {}", error_text).into())
    }
}

// Register with the discovery service
async fn register_with_discovery_service(
    node: &Node,
    uuid: String,
    address: String
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let discovery_service_url = format!("{}/register_node", DISCOVERY_SERVICE_URL);
    info!("Address: {}", address);
    let request_body = RegisterNodeRequest {
        id: uuid,
        address: address,
        public_key: node.public_key.clone(),
    };

    let response = client.post(&discovery_service_url).json(&request_body).send().await;

    match response {
        Ok(resp) => {
            let status = resp.status(); // Store the status code before consuming the response
            if status.is_success() {
                println!("Successfully registered with discovery service.");
                Ok(())
            } else {
                let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                eprintln!("Failed to register with discovery service. Status: {}", status);
                eprintln!("API error body: {}", error_text);
                Err(format!("API error: {}", status).into())
            }
        }
        Err(e) => {
            eprintln!("Failed to connect to discovery service: {}", e);
            Err(e.into())
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::DEBUG).init();

    info!("Starting application...");

    // Load the configuration (UUID and address)
    let config = Config::load(CONFIG_FILE).expect("Failed to create or fetch UUID and address");

    // Fetch and update nodes from the discovery service
    let node_info = fetch_and_update_nodes(NODE_INFO_FILE).await.unwrap_or_else(|_| NodeInfo {
        nodes: vec![],
    });

    // Initialize NodeList and determine if the current node needs to register
    let node_list = Arc::new(tokio::sync::Mutex::new(NodeList::from_nodes(node_info.nodes)));

    let node = {
        let mut node_list_guard = node_list.lock().await;
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

    let storage = Arc::new(tokio::sync::Mutex::new(Storage::new("database/db")));

    // Periodically fetch and update nodes every 5 seconds
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

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(Arc::clone(&node_list)))
            .app_data(web::Data::new(Arc::clone(&storage)))
            .configure(api::init_routes)
    })
        .bind(config.address.parse::<SocketAddr>().expect("Invalid socket address"))?
        .run().await
}

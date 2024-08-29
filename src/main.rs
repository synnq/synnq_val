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
use std::error::Error;
use crate::node::{ NodeList, Node };
use std::io::{ self, Write };
use uuid::Uuid;
use config::Config;

const CONFIG_FILE: &str = "config.json";

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

// Register with the discovery service
async fn register_with_discovery_service(node: &Node, id: String) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let discovery_service_url = "https://synnq-discovery-f77aaphiwa-uc.a.run.app/register_node";

    let request_body = RegisterNodeRequest {
        id: id,
        address: node.address.clone(),
        public_key: node.public_key.clone(),
    };

    let response = client.post(discovery_service_url).json(&request_body).send().await?;

    if response.status().is_success() {
        println!("Successfully registered with discovery service.");
    } else {
        eprintln!("Failed to register with discovery service. Status: {}", response.status());
    }

    Ok(())
}

// Fetch and update nodes from discovery service
async fn fetch_and_update_nodes(node_info_file: &str) -> Result<NodeInfo, Box<dyn Error>> {
    let client = Client::new();
    let discovery_service_url = "https://synnq-discovery-f77aaphiwa-uc.a.run.app/nodes";

    // Send a GET request to the discovery service
    let response = client.get(discovery_service_url).send().await?;
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let node_info_file = "node_info.json";

    // Fetch and update nodes from the discovery service
    let node_info = fetch_and_update_nodes(node_info_file).await.unwrap_or_else(|_| NodeInfo {
        nodes: vec![],
    });

    // Load the current node information from the file
    let node_list = NodeList::from_nodes(node_info.nodes);

    // Check for UUID in config and handle registration if needed
    let uuid = check_or_create_uuid().unwrap();

    let node = if let Some(existing_node) = node_list.find_node_by_uuid(&uuid) {
        existing_node
    } else {
        let new_node = Node::new("127.0.0.1:8080"); // Or get the external IP as before
        register_with_discovery_service(&new_node, uuid.clone()).await.unwrap();
        node_list.add_node(new_node.clone());
        new_node
    };

    println!("Node ID: {}", node.id);
    println!("Public Key: {}", node.public_key);

    let storage = storage::Storage::new("database/db");

    HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new(node_list.clone()))
            .app_data(actix_web::web::Data::new(storage.clone()))
            .configure(api::init_routes)
    })
        .bind(("127.0.0.1", 8080))?
        .run().await
}

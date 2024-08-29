mod node;
mod api;
mod validation;
mod consensus;
mod storage;

use actix_web::{ App, HttpServer };
use reqwest::Client;
use serde::{ Deserialize, Serialize };
use std::{ fs, error::Error, time::Duration };
use uuid::Uuid;
use tokio::time::timeout;

#[derive(Serialize, Deserialize)]
struct Config {
    uuid: String,
}

#[derive(Serialize)]
struct RegisterNodeRequest {
    id: String,
    address: String,
    public_key: String,
}

#[derive(Serialize, Deserialize)]
struct NodeInfo {
    nodes: Vec<node::Node>,
}

async fn register_with_discovery_service(node: &node::Node) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let discovery_service_url = "https://synnq-discovery-f77aaphiwa-uc.a.run.app/register_node";

    let request_body = RegisterNodeRequest {
        id: node.id.clone(),
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

async fn fetch_and_update_nodes(node_info_file: &str) -> Result<(), Box<dyn Error>> {
    let client = Client::new();
    let discovery_service_url = "https://synnq-discovery-f77aaphiwa-uc.a.run.app/nodes";

    let response = client.get(discovery_service_url).send().await?;
    println!("Response: {:?}", response);

    if response.status().is_success() {
        let nodes: Vec<node::Node> = response.json().await?;
        let node_info = NodeInfo { nodes };

        // Save the nodes to node_info.json
        let data = serde_json::to_string_pretty(&node_info)?;
        fs::write(node_info_file, data)?;

        println!("Node information updated successfully.");
    } else {
        eprintln!("Failed to fetch nodes from discovery service. Status: {}", response.status());
    }

    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config_file = "config.json";
    let node_info_file = "node_info.json";

    // Step 2: Check if config.json has a UUID
    let uuid = if let Ok(contents) = fs::read_to_string(config_file) {
        match serde_json::from_str::<Config>(&contents) {
            Ok(config) => config.uuid,
            Err(_) => {
                eprintln!("Failed to parse config.json. Generating new UUID.");
                let new_uuid = Uuid::new_v4().to_string();
                let new_config = Config { uuid: new_uuid.clone() };
                fs::write(config_file, serde_json::to_string_pretty(&new_config)?)?;
                new_uuid
            }
        }
    } else {
        eprintln!("config.json not found. Generating new UUID.");
        let new_uuid = Uuid::new_v4().to_string();
        let new_config = Config { uuid: new_uuid.clone() };
        fs::write(config_file, serde_json::to_string_pretty(&new_config)?)?;
        new_uuid
    };

    // Step 2.2: Register or verify node with the discovery service
    let mut node_list = node::NodeList::new();
    fetch_and_update_nodes(node_info_file).await.ok();

    let node = if let Some(existing_node) = node_list.find_node_by_uuid(&uuid) {
        existing_node
    } else {
        eprintln!("Node with UUID {} not found. Registering new node.", uuid);
        let new_node = node::Node::new("127.0.0.1:8080"); // Replace with the actual IP
        register_with_discovery_service(&new_node).await.ok();
        fetch_and_update_nodes(node_info_file).await.ok();
        new_node
    };

    println!("Node ID: {}", node.id);
    println!("Public Key: {}", node.public_key);

    node_list.add_node(node.clone());

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

use reqwest::Client;
use serde::{ Deserialize, Serialize };
use std::fs;
use std::error::Error;
use uuid::Uuid;
use tokio::time::Duration;
use std::net::SocketAddr;
use crate::node::node::Node;
use crate::config::Config;
use tracing::info;
use anyhow::{ anyhow, Result };

const CONFIG_FILE: &str = "config.json";
const DISCOVERY_SERVICE_URL: &str = "https://synnq-discovery-f77aaphiwa-uc.a.run.app";

#[derive(Serialize)]
struct RegisterNodeRequest {
    id: String,
    address: String,
    public_key: String,
}

#[derive(Serialize, Deserialize)]
pub struct NodeInfo {
    pub nodes: Vec<Node>,
}

// Check if config.json exists, and create if not. Check or create UUID.
pub fn check_or_create_uuid() -> std::io::Result<Config> {
    if let Ok(config_data) = fs::read_to_string(CONFIG_FILE) {
        if let Ok(config) = serde_json::from_str::<Config>(&config_data) {
            return Ok(config);
        }
    }

    let new_uuid = Uuid::new_v4().to_string();

    println!("Enter the node's address (e.g., 127.0.0.1:8080): ");
    let input_address = read_address_from_user()?;

    let new_config = Config {
        uuid: new_uuid,
        address: input_address,
    };

    let config_json = serde_json::to_string_pretty(&new_config)?;
    fs::write(CONFIG_FILE, config_json)?;

    Ok(new_config)
}

fn read_address_from_user() -> std::io::Result<String> {
    let mut input_address = String::new();
    std::io::stdin().read_line(&mut input_address)?;
    let input_address = input_address.trim().to_string();

    if input_address.parse::<SocketAddr>().is_err() {
        eprintln!("Invalid address format: {}", input_address);
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid address format"));
    }

    Ok(input_address)
}

pub async fn resolve_address(address: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    for attempt in 1..=5 {
        match reqwest::get(address).await {
            Ok(_) => {
                println!("Successfully connected to {}", address);
                return Ok(());
            }
            Err(e) => {
                eprintln!("Error resolving address (attempt {}): {:?}", attempt, e);
                tokio::time::sleep(Duration::from_secs(5 * attempt)).await; // Exponential backoff
            }
        }
    }

    Err(
        Box::new(
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to resolve address after multiple attempts"
            )
        )
    )
}

// Fetch and update nodes from the discovery service
pub async fn fetch_and_update_nodes(
    node_info_file: &str
) -> Result<NodeInfo, Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let discovery_service_url = format!("{}/nodes", DISCOVERY_SERVICE_URL);

    let response = client.get(&discovery_service_url).send().await?;

    if response.status().is_success() {
        let nodes: Vec<Node> = response.json().await?;
        let node_info = NodeInfo { nodes };

        let data = serde_json::to_string_pretty(&node_info)?;
        fs::write(node_info_file, data)?;

        println!("Node information updated successfully.");
        Ok(node_info)
    } else {
        eprintln!("Failed to fetch nodes from discovery service. Status: {}", response.status());
        let error_text = response.text().await?;
        Err(format!("API error body: {}", error_text).into())
    }
}

// Register with the discovery service
pub async fn register_with_discovery_service(
    node: &Node,
    uuid: String,
    address: String
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let discovery_service_url = format!("{}/register_node", DISCOVERY_SERVICE_URL);
    info!("Registering node with address: {}", address);

    let request_body = RegisterNodeRequest {
        id: uuid,
        address: address.clone(),
        public_key: node.public_key.clone(),
    };

    let response = client.post(&discovery_service_url).json(&request_body).send().await?;

    let status = response.status(); // Capture the status code before consuming the response

    if status.is_success() {
        println!("Successfully registered with discovery service.");
        Ok(())
    } else {
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
        eprintln!("Failed to register with discovery service. Status: {}", status);
        eprintln!("API error body: {}", error_text);
        Err(anyhow!("API error: {}", status).into())
    }
}

use reqwest::Client;
use serde::{ Deserialize, Serialize };
use std::fs;
use std::error::Error;
use crate::node::node::Node;
use anyhow::{ anyhow, Result };

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

// Function to resolve the address
pub async fn resolve_address(address: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    for attempt in 1..=5 {
        match reqwest::get(address).await {
            Ok(_) => {
                println!("Successfully connected to {}", address);
                return Ok(());
            }
            Err(e) => {
                eprintln!("Error resolving address (attempt {}): {:?}", attempt, e);
                tokio::time::sleep(tokio::time::Duration::from_secs(5 * attempt)).await;
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

// Function to fetch and update nodes
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

// Function to register with the discovery service
pub async fn register_with_discovery_service(
    node: &Node,
    uuid: String,
    address: String
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let client = Client::new();
    let discovery_service_url = format!("{}/register_node", DISCOVERY_SERVICE_URL);

    let request_body = RegisterNodeRequest {
        id: uuid,
        address: address.clone(),
        public_key: node.public_key.clone(),
    };

    let response = client.post(&discovery_service_url).json(&request_body).send().await?;

    let status = response.status();

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

mod node;
mod api;
mod validation;
mod consensus;
mod storage;

use actix_web::{ App, HttpServer };
use reqwest::Client;
use serde::{ Serialize, Deserialize };
use std::fs;
use std::error::Error;
use std::io::{ self, Write };
use uuid::Uuid;

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

#[derive(Serialize, Deserialize)]
struct Config {
    uuid: String,
}

impl Config {
    fn load(config_file: &str) -> Result<Self, std::io::Error> {
        match fs::read_to_string(config_file) {
            Ok(contents) =>
                serde_json
                    ::from_str(&contents)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)),
            Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
                let new_config = Config {
                    uuid: Uuid::new_v4().to_string(),
                };
                new_config.save(config_file)?;
                Ok(new_config)
            }
            Err(e) => Err(e),
        }
    }

    fn save(&self, config_file: &str) -> Result<(), std::io::Error> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write(config_file, data)?;
        Ok(())
    }
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

        let data = serde_json::to_string_pretty(&node_info)?;
        fs::write(node_info_file, &data)?;

        println!("Node information updated successfully.");
    } else {
        eprintln!("Failed to fetch nodes from discovery service. Status: {}", response.status());
    }

    Ok(())
}

fn prompt_for_external_ip() -> String {
    print!("Enter the external IP address: ");
    io::stdout().flush().unwrap();
    let mut ip = String::new();
    io::stdin().read_line(&mut ip).unwrap();
    ip.trim().to_string()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let config_file = "config.json";
    let config = Config::load(config_file).expect("Failed to load or create config file");

    let node_info_file = "node_info.json";

    // Fetch and update nodes from the discovery service
    let _ = fetch_and_update_nodes(node_info_file).await;

    // Load the current node information from the file
    let node_list = if fs::metadata(node_info_file).is_ok() {
        match fs::read_to_string(node_info_file) {
            Ok(contents) => {
                match serde_json::from_str::<NodeInfo>(&contents) {
                    Ok(node_info) => {
                        let mut node_list = node::NodeList::new();
                        for node in node_info.nodes {
                            node_list.add_node(node);
                        }
                        node_list
                    }
                    Err(_) => {
                        eprintln!("Failed to parse node info. Creating a new node list.");
                        node::NodeList::new()
                    }
                }
            }
            Err(_) => {
                eprintln!("Failed to read node info file. Creating a new node list.");
                node::NodeList::new()
            }
        }
    } else {
        node::NodeList::new()
    };

    // Check if the UUID from the config exists in the node list
    let node = if
        let Some(existing_node) = node_list
            .get_nodes()
            .iter()
            .find(|n| n.id == config.uuid)
    {
        existing_node.clone()
    } else {
        let external_ip = prompt_for_external_ip();
        let node_address = format!("{}:8080", external_ip);
        let mut new_node = node::Node::new(&node_address);
        new_node.id = config.uuid.clone();
        new_node.save_to_file(node_info_file);
        node_list.add_node(new_node.clone());

        // Register the new node with the discovery service
        if let Err(e) = register_with_discovery_service(&new_node).await {
            eprintln!("Error during registration: {}", e);
        }

        new_node
    };

    println!("Node ID: {}", node.id);
    println!("Public Key: {}", node.public_key);

    let storage = storage::Storage::new("database/db");

    HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new(node_list.clone()))
            .app_data(actix_web::web::Data::new(storage.clone())) // Pass the shared storage instance
            .configure(api::init_routes)
    })
        .bind(("0.0.0.0", 8080))
        ? // Bind to all interfaces
        .run().await
}

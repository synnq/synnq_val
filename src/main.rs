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
        fs::write(node_info_file, &data)?;
        println!("Data saved to {:?}", data);

        println!("Node information updated successfully.");
    } else {
        eprintln!("Failed to fetch nodes from discovery service. Status: {}", response.status());
    }

    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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

    // Create a new node and save it if needed
    let node = if node_list.get_nodes().is_empty() {
        let new_node = node::Node::new("127.0.0.1:8080");
        new_node.save_to_file(node_info_file);
        node_list.add_node(new_node.clone());
        new_node
    } else {
        node_list.get_nodes()[0].clone()
    };

    println!("Node ID: {}", node.id);
    println!("Public Key: {}", node.public_key);

    // Register the node with the discovery service
    if let Err(e) = register_with_discovery_service(&node).await {
        eprintln!("Error during registration: {}", e);
    }

    let storage = storage::Storage::new("database/db");

    HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new(node_list.clone()))
            .app_data(actix_web::web::Data::new(storage.clone())) // Pass the shared storage instance
            .configure(api::init_routes)
    })
        .bind(("127.0.0.1", 8080))?
        .run().await
}

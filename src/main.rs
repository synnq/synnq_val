mod node;
mod api;
mod validation;
mod consensus;
mod storage;

use actix_web::{ App, HttpServer };
use std::fs;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Check if node information already exists, otherwise create new
    let node_info_file = "node_info.json";
    let node = if fs::metadata(node_info_file).is_ok() {
        // Load existing node information
        node::Node::load_from_file(node_info_file).expect("Failed to load node info")
    } else {
        // Create a new node
        let new_node = node::Node::new("127.0.0.1:8080");
        new_node.save_to_file(node_info_file);
        new_node
    };

    // Log the node's unique ID and public key
    println!("Node ID: {}", node.id);
    // println!("Public Key: {}", node.public_key);

    let node_list = node::NodeList::new();
    node_list.add_node(node.clone());

    let storage = storage::Storage::new("database/db"); // Create a single shared instance

    HttpServer::new(move || {
        App::new()
            .app_data(actix_web::web::Data::new(node_list.clone()))
            .app_data(actix_web::web::Data::new(storage.clone())) // Pass the shared storage instance
            .configure(api::init_routes)
    })
        .bind(("127.0.0.1", 8080))?
        .run().await
}

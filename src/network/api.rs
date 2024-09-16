use actix_web::{ web, Responder, post, get, HttpResponse, Error };

use serde::{ Deserialize, Serialize };
use serde_json::Value;
use crate::{ node::node::{ Node, NodeList }, consensus::handle_validation, storage::Storage };
use crate::validation::validate_data;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Data {
    pub secret: String,
    pub data: Value,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RegisterNodeRequest {
    pub id: String,
    pub address: String,
    pub public_key: String,
}

#[post("/register_node")]
async fn register_node(
    req: web::Json<RegisterNodeRequest>,
    node_list: web::Data<Arc<Mutex<NodeList>>>
) -> impl Responder {
    let node_list = node_list.lock().await;

    let node = Node {
        id: req.id.clone(),
        address: req.address.clone(),
        public_key: req.public_key.clone(),
        validated: Some(false),
    };

    node_list.add_node(node);
    format!("Node {} registered successfully", req.id)
}

#[get("/nodes")]
async fn get_nodes(node_list: web::Data<Arc<Mutex<NodeList>>>) -> impl Responder {
    let node_list = node_list.lock().await;
    let nodes = node_list.get_nodes();
    web::Json(nodes)
}

#[post("/receive_data")]
async fn receive_data(
    data: web::Json<Data>,
    node_list: web::Data<Arc<Mutex<NodeList>>>,
    storage: web::Data<Arc<Mutex<Storage>>>
) -> Result<HttpResponse, Error> {
    // Avoid holding the lock across async boundaries
    let nodes = {
        let node_list = node_list.lock().await;
        node_list.get_nodes().clone()
    };

    // Validate the data using the first node
    if !validate_data(&nodes[0], &data.data).await {
        return Ok(HttpResponse::BadRequest().body("Invalid data structure in `data` field"));
    }

    // Perform validation and broadcast
    handle_validation(data.into_inner(), node_list.clone(), storage.clone()).await
}

#[post("/receive_broadcast")]
async fn receive_broadcast(
    transaction_data: web::Json<Value>,
    storage: web::Data<Arc<Mutex<Storage>>>
) -> impl Responder {
    println!("Received broadcasted transaction data: {:?}", transaction_data);

    let storage_key = "broadcasted_transaction";

    {
        let storage = storage.lock().await;
        storage.store_data(storage_key, &transaction_data.to_string());
    }

    HttpResponse::Ok().body("Broadcast received successfully")
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ProxyRequest {
    pub target_url: String,
    pub data: Value,
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(register_node);
    cfg.service(get_nodes);
    cfg.service(receive_data);
    cfg.service(receive_broadcast);
}

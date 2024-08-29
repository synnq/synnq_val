use actix_web::{ web, Responder, post, get, HttpResponse, Error };
use serde::{ Deserialize, Serialize };
use serde_json::Value;
use crate::{
    node::{ Node, NodeList },
    consensus::{ handle_validation, broadcast_to_nodes },
    storage::Storage,
};
use crate::validation::validate_data;
use std::sync::{ Arc, Mutex };

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Data {
    pub secret: u64,
    pub proof: Vec<u8>,
    pub blinding: String,
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
    let node = Node {
        id: req.id.clone(),
        address: req.address.clone(),
        public_key: req.public_key.clone(),
        validated: Some(false),
    };

    node_list.lock().unwrap().add_node(node);
    format!("Node {} registered successfully", req.id)
}

#[get("/nodes")]
async fn get_nodes(node_list: web::Data<Arc<Mutex<NodeList>>>) -> impl Responder {
    let nodes = node_list.lock().unwrap().get_nodes();
    web::Json(nodes)
}

#[post("/receive_data")]
async fn receive_data(
    data: web::Json<Data>,
    node_list: web::Data<Arc<Mutex<NodeList>>>,
    storage: web::Data<Arc<Mutex<Storage>>>
) -> Result<HttpResponse, Error> {
    let node_list_clone = node_list.clone(); // Clone the node_list to avoid moving it
    let storage_clone = storage.clone(); // Clone the storage to avoid moving it

    if !validate_data(&node_list.lock().unwrap().get_nodes()[0], &data.data).await {
        return Ok(HttpResponse::BadRequest().body("Invalid data structure in `data` field"));
    }

    handle_validation(data.into_inner(), node_list_clone, storage_clone).await
}

#[post("/receive_broadcast")]
async fn receive_broadcast(
    transaction_data: web::Json<Value>,
    storage: web::Data<Arc<Mutex<Storage>>>
) -> impl Responder {
    println!("Received broadcasted transaction data: {:?}", transaction_data);

    let storage_key = "broadcasted_transaction";
    storage.lock().unwrap().store_data(storage_key, &transaction_data.to_string());

    HttpResponse::Ok().body("Broadcast received successfully")
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(register_node);
    cfg.service(get_nodes);
    cfg.service(receive_data);
    cfg.service(receive_broadcast);
}

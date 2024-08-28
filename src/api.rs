use actix_web::{ web, Responder, post, get, HttpResponse, Error };
use serde::{ Deserialize, Serialize };
use serde_json::Value;
use crate::{ node::{ Node, NodeList }, consensus::handle_validation, storage::Storage };
use crate::validation::validate_data;

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Data {
    pub secret: u64, // secret is a number (integer)
    pub proof: Vec<u8>, // proof is a vector of unsigned 8-bit integers
    pub blinding: String, // blinding is a hexadecimal string
    pub data: Value, // data is a JSON object, using serde_json::Value
}

// Struct for node registration request
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RegisterNodeRequest {
    pub id: String,
    pub address: String,
    pub public_key: String, // Add the public_key field here
}

#[post("/register_node")]
async fn register_node(
    req: web::Json<RegisterNodeRequest>,
    node_list: web::Data<NodeList>
) -> impl Responder {
    let node = Node {
        id: req.id.clone(),
        address: req.address.clone(),
        public_key: req.public_key.clone(),
        validated: Some(false), // Corrected line
    };

    node_list.add_node(node);
    format!("Node {} registered successfully", req.id)
}

#[get("/nodes")]
async fn get_nodes(node_list: web::Data<NodeList>) -> impl Responder {
    let nodes = node_list.get_nodes();
    web::Json(nodes)
}

#[post("/receive_data")]
async fn receive_data(
    data: web::Json<Data>,
    node_list: web::Data<NodeList>,
    storage: web::Data<Storage>
) -> Result<HttpResponse, Error> {
    // Validate the `data` field within the Data struct
    if !validate_data(&node_list.get_nodes()[0], &data.data).await {
        return Ok(HttpResponse::BadRequest().body("Invalid data structure in `data` field"));
    }

    // If validation passes, proceed with handling validation
    handle_validation(data.into_inner(), node_list, storage).await
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(register_node);
    cfg.service(get_nodes);
    cfg.service(receive_data);
}

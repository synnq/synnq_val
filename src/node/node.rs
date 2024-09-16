use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use uuid::Uuid;
use rand::rngs::OsRng;
use rsa::{RsaPrivateKey, RsaPublicKey};
use rsa::pkcs1::{EncodeRsaPrivateKey, EncodeRsaPublicKey, LineEnding};  // Import the traits
use std::fs;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub address: String,
    pub public_key: String,
    pub validated: Option<bool>,
}

#[derive(Clone)]
pub struct NodeList {
    nodes: Arc<Mutex<HashMap<String, Node>>>,
}

impl NodeList {
    pub fn new() -> Self {
        NodeList {
            nodes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn from_nodes(nodes: Vec<Node>) -> Self {
        let node_list = NodeList::new();
        for node in nodes {
            node_list.add_node(node);
        }
        node_list
    }

    pub fn add_node(&self, node: Node) {
        self.nodes.lock().unwrap().insert(node.id.clone(), node);
    }

    pub fn get_nodes(&self) -> Vec<Node> {
        self.nodes.lock().unwrap().values().cloned().collect()
    }

    pub fn find_node_by_uuid(&self, uuid: &str) -> Option<Node> {
        self.nodes.lock().unwrap().get(uuid).cloned()
    }

    pub fn remove_node_by_uuid(&self, uuid: &str) -> bool {
        let mut nodes = self.nodes.lock().unwrap();
        if nodes.remove(uuid).is_some() {
            true
        } else {
            false
        }
    }
}

impl Node {
    pub fn new(address: &str) -> Self {
        let id = Uuid::new_v4().to_string();

        let mut rng = OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("failed to generate a key");
        let public_key = RsaPublicKey::from(&private_key);

        let private_key_pem = private_key.to_pkcs1_pem(LineEnding::LF).unwrap(); // For private key
        let public_key_pem = public_key.to_pkcs1_pem(LineEnding::LF).unwrap();   // For public key

        fs::write("private_key.pem", private_key_pem.as_bytes()).expect("unable to write private key");

        Node {
            id,
            address: address.to_string(),
            public_key: public_key_pem,
            validated: Some(false),
        }
    }
}
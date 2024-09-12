use std::collections::HashMap;
use std::sync::{ Arc, Mutex };
use serde::{ Serialize, Deserialize };
use uuid::Uuid;
use rand::rngs::OsRng;
use rsa::{ RsaPrivateKey, RsaPublicKey };
use rsa::pkcs1::{ ToRsaPrivateKey, ToRsaPublicKey };
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
    // Create a new NodeList with an empty HashMap
    pub fn new() -> Self {
        NodeList {
            nodes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // Create a NodeList from a Vec<Node>
    pub fn from_nodes(nodes: Vec<Node>) -> Self {
        let node_list = NodeList::new();
        for node in nodes {
            node_list.add_node(node);
        }
        node_list
    }

    // Add a new node to the NodeList
    pub fn add_node(&self, node: Node) {
        self.nodes.lock().unwrap().insert(node.id.clone(), node);
    }

    // Retrieve all nodes as a Vec<Node>
    pub fn get_nodes(&self) -> Vec<Node> {
        self.nodes.lock().unwrap().values().cloned().collect()
    }

    // Find a node by UUID
    pub fn find_node_by_uuid(&self, uuid: &str) -> Option<Node> {
        self.nodes.lock().unwrap().get(uuid).cloned()
    }

    // Remove a node by its UUID and return true if successful
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
    // Create a new Node instance with a generated UUID and RSA key pair
    pub fn new(address: &str) -> Self {
        // Generate a unique ID for the node
        let id = Uuid::new_v4().to_string();

        // Generate an RSA key pair
        let mut rng = OsRng;
        let private_key = RsaPrivateKey::new(&mut rng, 2048).expect("failed to generate a key");
        let public_key = RsaPublicKey::from(&private_key);

        // Store the public and private keys as PEM strings
        let private_key_pem = private_key.to_pkcs1_pem().unwrap();
        let public_key_pem = public_key.to_pkcs1_pem().unwrap();

        // Save the private key to a file for later use
        fs::write("private_key.pem", private_key_pem.as_bytes()).expect(
            "unable to write private key"
        );

        // Return the new Node instance
        Node {
            id,
            address: address.to_string(),
            public_key: public_key_pem, // No need for into_inner(), already a String
            validated: Some(false),
        }
    }
}

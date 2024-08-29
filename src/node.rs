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
    pub fn new() -> Self {
        NodeList {
            nodes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add_node(&self, node: Node) {
        self.nodes.lock().unwrap().insert(node.id.clone(), node);
    }

    pub fn get_nodes(&self) -> Vec<Node> {
        self.nodes.lock().unwrap().values().cloned().collect()
    }

    pub fn update_validation(&self, node_id: &str, validated: bool) {
        if let Some(node) = self.nodes.lock().unwrap().get_mut(node_id) {
            node.validated = Some(validated); // Wrap the boolean value in `Some`
        }
    }

    pub fn find_node_by_uuid(&self, uuid: &str) -> Option<Node> {
        self.nodes.lock().unwrap().get(uuid).cloned()
    }
}

impl Node {
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

    pub fn load_from_file(filename: &str) -> Option<Self> {
        let data = fs::read_to_string(filename).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn save_to_file(&self, filename: &str) {
        let data = serde_json::to_string(self).expect("unable to serialize node");
        fs::write(filename, data).expect("unable to write node data to file");
    }
}

use crate::node::Node;
use serde::{ Deserialize, Serialize };
use serde_json::from_str;

use serde_json::{ Value, from_value };

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TransactionData {
    pub transaction_type: String,
    pub sender: String,
    pub private_key: String,
    pub receiver: String,
    pub amount: u64,
    pub denom: String,
    pub fee: u64,
    pub flags: u64,
    pub data_type: String,
    pub data: DataField,
    pub metadata: Metadata,
    pub model_type: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DataField {
    pub data: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Metadata {
    pub meta: Meta,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Meta {
    pub value: String,
}

// The validation function
pub async fn validate_data(_node: &Node, data: &Value) -> bool {
    // Attempt to deserialize the `data` field into the `TransactionData` struct
    match from_value::<TransactionData>(data.clone()) {
        Ok(_) => true, // If deserialization succeeds, the data is valid
        Err(_) => false, // If deserialization fails, the data is invalid
    }
}

use crate::node::Node;
use serde::{ Deserialize, Serialize };
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

pub async fn validate_data(_node: &Node, data: &Value) -> bool {
    match from_value::<TransactionData>(data.clone()) {
        Ok(transaction) => {
            if transaction.transaction_type.is_empty() {
                eprintln!("Transaction type is empty");
                return false;
            }

            if transaction.sender.is_empty() || transaction.receiver.is_empty() {
                eprintln!("Sender or receiver address is empty");
                return false;
            }

            if transaction.private_key.len() != 64 {
                eprintln!("Invalid private key length");
                return false;
            }

            true
        }
        Err(e) => {
            eprintln!("Failed to deserialize data: {}", e);
            false
        }
    }
}

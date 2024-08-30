use crate::node::node::Node;
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
    println!("Validating data: {:#?}", data);

    match from_value::<TransactionData>(data.clone()) {
        Ok(transaction) => {
            if transaction.transaction_type.is_empty() {
                println!("Validation failed: Transaction type is empty");
                return false;
            }

            if transaction.sender.is_empty() {
                println!("Validation failed: Sender address is empty");
                return false;
            }

            if transaction.receiver.is_empty() {
                println!("Validation failed: Receiver address is empty");
                return false;
            }

            if transaction.private_key.len() != 64 {
                println!("Validation failed: Invalid private key length");
                return false;
            }

            if transaction.amount == 0 {
                println!("Validation failed: Transaction amount is zero");
                return false;
            }

            if transaction.denom.is_empty() {
                println!("Validation failed: Denomination is empty");
                return false;
            }

            if transaction.fee == 0 {
                println!("Validation failed: Transaction fee is zero");
                return false;
            }

            if transaction.data.data.is_empty() {
                println!("Validation failed: Data field is empty");
                return false;
            }

            if transaction.metadata.meta.value.is_empty() {
                println!("Validation failed: Metadata value is empty");
                return false;
            }

            println!("Validation succeeded");
            true
        }
        Err(e) => {
            println!("Failed to deserialize data: {}", e);
            false
        }
    }
}

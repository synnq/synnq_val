use crate::{ node::node::NodeList, validation::validate_data, storage::Storage };
use crate::network::api::Data;
use actix_web::{ web, HttpResponse, Error };
use reqwest::Client;
use anyhow::{ anyhow, Result };
use futures::future::join_all;
use tokio::time::{ timeout, Duration };
use crate::node::node::Node;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::info;
use crate::config::Config;
use serde_json::{ json, Value };

pub async fn handle_validation(
    data: Data,
    node_list: web::Data<Arc<Mutex<NodeList>>>,
    storage: web::Data<Arc<Mutex<Storage>>>
) -> Result<HttpResponse, Error> {
    let nodes = {
        let node_list = node_list.lock().await;
        node_list.get_nodes().clone()
    };
    let mut validated_count = 0;

    println!("Starting validation with nodes: {:#?}", nodes);

    let validation_results: Vec<_> = join_all(
        nodes.iter().map(|node| async {
            let res = timeout(Duration::from_secs(5), validate_data(node, &data.data)).await;
            info!("Validating data from node {}: {:#?}", node.id, res);
            match res {
                Ok(true) => {
                    println!("Node {} successfully validated the data.", node.id);
                    Some(true)
                }
                Ok(false) => {
                    eprintln!("Node {} failed to validate the data.", node.id);
                    Some(false)
                }
                Err(_) => {
                    eprintln!("Node {} did not respond in time.", node.id);
                    None
                }
            }
        })
    ).await;

    for res in validation_results {
        if res.unwrap_or(false) {
            validated_count += 1;
        }
    }

    println!(
        "Validation passed: {} out of {} nodes successfully validated the data.",
        validated_count,
        nodes.len()
    );

    let required_percentage = 0.8;
    if (validated_count as f64) / (nodes.len() as f64) >= required_percentage {
        println!(
            "Validated Count {} >= Required Percentage {}",
            validated_count,
            required_percentage
        );

        if send_to_api(data.clone()).await {
            let storage_key = data.secret.to_string();
            storage.lock().await.store_data(&storage_key, &data.data.to_string());

            match send_transaction_data(&data.data).await {
                Ok(api_response) => {
                    if let Err(e) = broadcast_to_nodes(&nodes, &data.data).await {
                        eprintln!("Failed to broadcast to nodes: {}", e);
                    }
                    Ok(HttpResponse::Ok().body(api_response))
                }
                Err(e) => {
                    eprintln!("Failed to send transaction data: {}", e);
                    Ok(HttpResponse::InternalServerError().body("Failed to send transaction data"))
                }
            }
        } else {
            println!("Data validation failed on external API");
            Ok(HttpResponse::BadRequest().body("Data validation failed on external API"))
        }
    } else {
        Ok(HttpResponse::BadRequest().body("Insufficient nodes validated the data"))
    }
}

async fn send_to_api(data: Data) -> bool {
    let client = Client::new();
    let mut attempts = 3;
    let mut delay = Duration::from_secs(1);
    println!("Sending data to API: {:?}", data);
    while attempts > 0 {
        // Log the exact payload being sent
        println!("Sending data to API: {:?}", data);

        let response = client.post("https://zkp.synnq.io/verify").json(&data).send().await;

        match response {
            Ok(res) => {
                let status = res.status();
                let error_body = res
                    .text().await
                    .unwrap_or_else(|_| "Unable to read error body".to_string());

                if status.is_success() {
                    println!("Data validation successful on external API");
                    return true;
                } else if status == 422 {
                    eprintln!("Unprocessable Entity: Check the request body for errors.");
                    eprintln!("Response Body: {}", error_body);
                    return false; // No need to retry if it's a 422 error
                } else {
                    eprintln!("API returned error status: {}", status);
                    eprintln!("API error body: {}", error_body);
                }
            }
            Err(err) => {
                eprintln!("Failed to send data to API: {}", err);
            }
        }

        attempts -= 1;
        sleep(delay).await;
        delay *= 2;
    }

    println!("Data validation failed on external API after retries");
    false
}

async fn send_transaction_data(transaction_data: &Value) -> Result<String> {
    let client = Client::new();

    // Log the transaction_data to see its structure
    println!("Received transaction_data: {:#?}", transaction_data);

    // Step 1: Send the original transaction data
    let response = client
        .post("https://rest.synnq.io/transaction")
        .json(transaction_data)
        .send().await?;

    let status = response.status();
    let body = response.text().await?;

    if !status.is_success() {
        eprintln!("Failed to send transaction data. Status: {}", status);
        return Err(anyhow!("Failed to send transaction data. Status: {}. Body: {}", status, body));
    }

    println!("Transaction data successfully sent to https://rest.synnq.io/transaction");

    // Step 2: Extract fields from the transaction_data

    // Extract fees
    let fees_amount = transaction_data
        .get("fee")
        .ok_or_else(|| {
            eprintln!("Missing 'fees' field in transaction_data");
            anyhow!("Failed to extract fees from transaction data")
        })?
        .as_f64()
        .ok_or_else(|| {
            eprintln!("'fees' field is not a valid number");
            anyhow!("Fees is not a valid number")
        })?;

    // Extract sender
    let sender = transaction_data
        .get("sender")
        .ok_or_else(|| {
            eprintln!("Missing 'sender' field in transaction_data");
            anyhow!("Failed to extract sender from transaction data")
        })?
        .as_str()
        .ok_or_else(|| {
            eprintln!("'sender' field is not a valid string");
            anyhow!("Sender is not a valid string")
        })?;

    // Extract private_key
    let private_key = transaction_data
        .get("private_key")
        .ok_or_else(|| {
            eprintln!("Missing 'private_key' field in transaction_data");
            anyhow!("Failed to extract private key from transaction data")
        })?
        .as_str()
        .ok_or_else(|| {
            eprintln!("'private_key' field is not a valid string");
            anyhow!("Private key is not a valid string")
        })?;

    // Extract denom
    let denom = transaction_data
        .get("denom")
        .ok_or_else(|| {
            eprintln!("Missing 'denom' field in transaction_data");
            anyhow!("Failed to extract denom from transaction data")
        })?
        .as_str()
        .ok_or_else(|| {
            eprintln!("'denom' field is not a valid string");
            anyhow!("Denom is not a valid string")
        })?;

    // Step 3: Load wallet address from the config
    let config = Config::load("config.json")?;
    let wallet_address = config.wallet_address.ok_or_else(|| {
        eprintln!("Wallet address is not set in the configuration file");
        anyhow!("Wallet address is not set in the configuration file")
    })?;

    // Step 4: Create the request body for the fee transaction
    let fee_transaction_request =
        json!({
        "transaction_type": "payment",
        "sender": sender,  // Sender from the original transaction
        "private_key": private_key,  // Private key from the original transaction
        "receiver": wallet_address,  // Wallet address from config
        "amount": fees_amount,  // Fees extracted from the original transaction
        "fee": 0,
        "denom": denom,  // Denomination of the currency
        "flags": 1,  // Flags is set to 1
        "data_type": "fees",  // Data type set to "fees"
        "data": {
            "value": ""  // Optional data, modify if needed
        },
        "metadata": {
            "meta": {
                "value": ""  // Optional metadata, modify if needed
            }
        },
        "model_type": "default_model"  // Model type
    });

    // Step 5: Send the new fee transaction
    let fee_response = client
        .post("https://rest.synnq.io/transaction")
        .json(&fee_transaction_request)
        .send().await?;

    let fee_status = fee_response.status();
    let fee_body = fee_response.text().await?;

    if fee_status.is_success() {
        println!("Fee transaction successfully sent to wallet: {}", wallet_address);
        Ok(body)
    } else {
        eprintln!("Failed to send fee transaction. Status: {}", fee_status);
        Err(anyhow!("Failed to send fee transaction. Status: {}. Body: {}", fee_status, fee_body))
    }
}

async fn broadcast_to_nodes(nodes: &[Node], transaction_data: &serde_json::Value) -> Result<()> {
    let client = Client::new();

    let broadcast_results: Vec<_> = join_all(
        nodes.iter().map(|node| async {
            let url = if
                node.address.starts_with("http://") ||
                node.address.starts_with("https://")
            {
                format!("{}/receive_broadcast", node.address.trim_end_matches('/'))
            } else {
                format!("http://{}/receive_broadcast", node.address)
            };

            println!("Broadcasting to node {}: {}", node.id, url);

            match client.post(&url).json(transaction_data).send().await {
                Ok(res) if res.status().is_success() => {
                    println!("Broadcast to node {} succeeded.", node.id);
                    Ok(())
                }
                Ok(res) => {
                    eprintln!("Failed to broadcast to node {}. Status: {}", node.id, res.status());
                    Err(anyhow!("Broadcast failed for node {}", node.id))
                }
                Err(err) => {
                    eprintln!("Failed to send broadcast to node {}: {}", node.id, err);
                    Err(anyhow!("Broadcast failed for node {}", node.id))
                }
            }
        })
    ).await;

    for result in broadcast_results {
        if let Err(e) = result {
            eprintln!("Broadcast error: {}", e);
        }
    }

    Ok(())
}

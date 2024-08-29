use crate::{ node::NodeList, validation::validate_data, storage::Storage };
use crate::api::Data;
use actix_web::{ web, HttpResponse, Error };
use reqwest::Client;
use serde::Deserialize;
use anyhow::{ anyhow, Result };
use crate::node::Node;
use futures::future::join_all;
use tokio::time::{ timeout, Duration };

#[derive(Deserialize)]
struct VerifyResponse {
    valid: bool,
}

pub async fn handle_validation(
    data: Data,
    node_list: web::Data<NodeList>,
    storage: web::Data<Storage>
) -> Result<HttpResponse, Error> {
    let nodes = node_list.get_nodes();
    let mut validated_count = 0;

    println!("Starting validation with nodes: {:?}", nodes);

    let validation_results: Vec<_> = join_all(
        nodes.iter().map(|node| async {
            let res = timeout(Duration::from_secs(5), validate_data(node, &data.data)).await;
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
            storage.store_data(&storage_key, &data.data.to_string());

            // Send data.data to the external API and handle response
            match send_transaction_data(&data.data).await {
                Ok(api_response) => {
                    // Broadcast the transaction data to all other nodes
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
    let response = client.post("http://zkp.synnq.io/verify").json(&data).send().await;

    match response {
        Ok(res) => {
            if let Ok(verify_response) = res.json::<VerifyResponse>().await {
                if verify_response.valid {
                    println!("Data validation successful on external API");
                    return true;
                }
            }
            println!("Data validation failed on external API");
            false
        }
        Err(err) => {
            eprintln!("Failed to send data to API: {}", err);
            false
        }
    }
}

async fn send_transaction_data(transaction_data: &serde_json::Value) -> Result<String> {
    let client = Client::new();
    let response = client
        .post("https://rest.synnq.io/transaction")
        .json(transaction_data) // Send the transaction data as JSON
        .send().await?;

    let status = response.status();
    let body = response.text().await?;

    if status.is_success() {
        println!("Transaction data successfully sent to https://rest.synnq.io/transaction");
        println!("Response: {}", body);
        Ok(body) // Return the successful response body
    } else {
        eprintln!("Failed to send transaction data. Status: {}", status);
        eprintln!("Response: {}", body);
        Err(anyhow!("Failed to send transaction data. Status: {}. Body: {}", status, body))
    }
}

async fn broadcast_to_nodes(nodes: &[Node], transaction_data: &serde_json::Value) -> Result<()> {
    let client = Client::new();

    let broadcast_results: Vec<_> = join_all(
        nodes.iter().map(|node| async {
            let url = format!("{}/receive_broadcast", node.address);
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

    // Collect and log any errors from broadcasting
    for result in broadcast_results {
        if let Err(e) = result {
            eprintln!("Broadcast error: {}", e);
        }
    }

    Ok(())
}

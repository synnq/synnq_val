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

async fn send_transaction_data(transaction_data: &serde_json::Value) -> Result<String> {
    let client = Client::new();
    let response = client
        .post("https://rest.synnq.io/transaction")
        .json(transaction_data)
        .send().await?;

    let status = response.status();
    let body = response.text().await?;

    if status.is_success() {
        println!("Transaction data successfully sent to https://rest.synnq.io/transaction");
        Ok(body)
    } else {
        eprintln!("Failed to send transaction data. Status: {}", status);
        Err(anyhow!("Failed to send transaction data. Status: {}. Body: {}", status, body))
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

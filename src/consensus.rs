use crate::{ node::NodeList, validation::validate_data, storage::Storage };
use crate::api::Data;
use actix_web::{ web, HttpResponse, Error };
use reqwest::Client;
use serde::Deserialize;
use anyhow::{ anyhow, Result };
use crate::node::Node;
use futures::future::join_all;

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
    println!("Starting validation with nodes: {:?}", nodes);

    // Validate data with all nodes concurrently and collect results
    let validation_results: Vec<_> = join_all(
        nodes.iter().map(|node| async {
            match validate_data(node, &data.data).await {
                true => {
                    println!("Node {} successfully validated the data.", node.id);
                    node_list.update_validation(&node.id, true);
                    Some(true)
                }
                false => {
                    println!("Node {} failed to validate the data.", node.id);
                    None
                }
            }
        })
    ).await;

    // Calculate the percentage of successful validations
    let validated_count = validation_results
        .iter()
        .filter(|&&res| res.is_some())
        .count();
    let required_percentage = 0.8;
    let validation_passed = (validated_count as f64) / (nodes.len() as f64) >= required_percentage;

    if validation_passed {
        println!(
            "Validation passed: {} out of {} nodes successfully validated the data.",
            validated_count,
            nodes.len()
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
        println!(
            "Validation failed: only {} out of {} nodes successfully validated the data.",
            validated_count,
            nodes.len()
        );
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

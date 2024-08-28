use crate::{ node::NodeList, validation::validate_data, storage::Storage };
use crate::api::Data;
use actix_web::{ web, HttpResponse, Error };
use reqwest::Client;
use reqwest::StatusCode;
use serde::Deserialize;
use anyhow::{ anyhow, Result };

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

    for node in nodes.iter() {
        if validate_data(node, &data.data).await {
            node_list.update_validation(&node.id, true);
            validated_count += 1;
        }
    }

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
                Ok(api_response) => Ok(HttpResponse::Ok().body(api_response)),
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
    let response = client.post("http://127.0.0.1:8000/verify").json(&data).send().await;

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

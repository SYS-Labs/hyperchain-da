use async_trait::async_trait;
use anyhow::{anyhow, Result};
use hex::{encode, decode};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fmt;
use std::sync::Arc;
use zksync_da_client::{types::{DAError, DispatchResponse, InclusionData}, DataAvailabilityClient};
use serde_json::Value;

#[derive(Clone, Deserialize, Serialize)]
struct RPCError {
    code: i32,
    message: String,
}

#[derive(Clone, Deserialize, Serialize)]
struct CreateBlobResponse {
    error: Option<RPCError>,
    result: BlobResult,
}

#[derive(Clone, Deserialize, Serialize)]
struct BlobResult {
    versionhash: String,
}

#[derive(Clone)]
pub struct SyscoinClient {
    client: Arc<Client>,
    rpc_url: String,
    user: String,
    password: String,
    poda_url: String, 
}

impl SyscoinClient {
    pub fn new() -> Self {
        SyscoinClient {
            client: Arc::new(Client::new()),
            rpc_url: "http://l1:8370".to_string(),
            user: "u".to_string(),
            password: "p".to_string(),
            poda_url: "http://poda.tanenbaum.io/vh/".to_string(),
        }
    }

    async fn call_rpc<T: for<'a> Deserialize<'a>>(&self, method: &str, params: serde_json::Value) -> Result<T> {
        let body = json!({
            "method": method,
            "params": params,
            "id": "1",
            "jsonrpc": "2.0"
        });

        let response = self.client
            .post(&self.rpc_url)
            .basic_auth(&self.user, Some(&self.password))
            .json(&body)
            .send()
            .await?;

        let parsed: T = response.json().await?;
        Ok(parsed)
    }
}

#[async_trait]
impl DataAvailabilityClient for SyscoinClient {
    async fn dispatch_blob(&self, _batch_number: u32, data: Vec<u8>) -> Result<DispatchResponse, DAError> {
        let data_hex = encode(&data);
        let params = json!({ "data": data_hex });
        let response: CreateBlobResponse = self.call_rpc("syscoincreatenevmblob", params).await.map_err(|e| DAError { error: anyhow!(e), is_retriable: false })?;

        if let Some(err) = response.error {
            return Err(DAError { error: anyhow!(err.message), is_retriable: false });
        }

        Ok(DispatchResponse {
            blob_id: response.result.versionhash,
        })
    }

    async fn get_inclusion_data(&self, blob_id: &str) -> Result<Option<InclusionData>, DAError> {
        let params = json!({ "versionhash_or_txid": blob_id[2..] });
        // Assuming `call_rpc` returns a `Result` with an owned `Value`
        let response: Value = self.call_rpc("getnevmblobdata", params).await.map_err(|e| DAError { error: anyhow!(e), is_retriable: false })?;
    
        // Clone the error value if there is one, so we don't need to borrow from `response`
        if let Some(error) = response["error"].as_object().cloned() {
            return Err(DAError { error: anyhow!(error["message"].as_str().unwrap_or("Unknown error")), is_retriable: false });
        }
    
        // Similarly clone the "data" string
        let data_string = response["result"]["data"].as_str().map(String::from);
        let data = data_string
            .as_deref()
            .map(decode)
            .transpose()
            .map_err(|e| DAError { error: anyhow!(e), is_retriable: false })?;
    
        match data {
            Some(bytes) => Ok(Some(InclusionData { data: bytes })),
            None => Ok(None),
        }
    }


    fn clone_boxed(&self) -> Box<dyn DataAvailabilityClient> {
        Box::new(self.clone())
    }

    fn blob_size_limit(&self) -> Option<usize> {
        // Assuming there's no known limit
        None
    }
}

impl fmt::Debug for SyscoinClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SyscoinClient")
         .field("rpc_url", &self.rpc_url)
         .finish()
    }
}
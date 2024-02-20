use frame_metadata::{v15::RuntimeMetadataV15, RuntimeMetadata};

use jsonrpsee::core::{
    client::{BatchResponse, ClientT},
    params::BatchRequestBuilder,
    traits::ToRpcParams,
};
use jsonrpsee::types::{request::Request, Params};
use jsonrpsee::{rpc_params, server::Server, ws_client::WsClientBuilder, RpcModule};

use parity_scale_codec::Decode;

use primitive_types::H256;

use serde::Deserialize;
use serde_json::{value::Value, Map, Number};

use smoldot_light::{
    platform::DefaultPlatform, AddChainConfig, AddChainSuccess, ChainId, Client, JsonRpcResponses,
};

use std::{iter, num::NonZeroU32, sync::Arc};

use tokio::{
    sync::{broadcast, mpsc},
    time::{sleep, Duration},
};

/*
struct SmoldotClient {

}

impl ClientT for SmoldotClient {
    /// Send a [notification request](https://www.jsonrpc.org/specification#notification)
    async fn notification<Params>(&self, method: &str, params: Params) -> Result<(), Error>
        where
            Params: ToRpcParams + Send,
        {
            let params = params.to_rpc_params()?;
        }

    /// Send a [method call request](https://www.jsonrpc.org/specification#request_object).
    async fn request<R, Params>(&self, method: &str, params: Params) -> Result<Value, Error> {
        }

    /// Send a [batch request](https://www.jsonrpc.org/specification#batch).
    ///
    /// The response to batch are returned in the same order as it was inserted in the batch.
    ///
    ///
    /// Returns `Ok` if all requests in the batch were answered.
    /// Returns `Error` if the network failed or any of the responses could be parsed a valid JSON-RPC response.
    async fn batch_request<'a, R>(&self, batch: BatchRequestBuilder<'a>) -> Result<BatchResponse<'a, Value>, Error> {
        }
}
*/

/// Abstraction to connect to chain
///
/// This should run asynchronously under the hood and provide easy synchronous observables
pub struct Blockchain {
    block_hash: H256,
    client: Client<Arc<DefaultPlatform>, ()>,
    genesis_hash: H256,
    id: ChainId,
    responses: JsonRpcResponses<Arc<DefaultPlatform>>,
    metadata: RuntimeMetadataV15,
    specs: Map<String, Value>
}

impl Blockchain {
    pub async fn new() -> Self {
        let mut client = Client::new(DefaultPlatform::new(
            env!("CARGO_PKG_NAME").into(),
            env!("CARGO_PKG_VERSION").into(),
        ));
        let chain_config = AddChainConfig {
            user_data: (),
            specification: include_str!("../../chain-specs/westend.json"),
            database_content: "",
            potential_relay_chains: iter::empty(),
            json_rpc: smoldot_light::AddChainConfigJsonRpc::Enabled {
                max_pending_requests: NonZeroU32::new(u32::max_value()).unwrap(),
                max_subscriptions: u32::max_value(),
            },
        };
        let AddChainSuccess {
            chain_id: id,
            json_rpc_responses: responses,
        } = client.add_chain(chain_config).unwrap();
        let mut responses = responses.unwrap();

        //let (request_id_tx, mut request_id_rx) = tokyo::

        /*
        tokio::spawn(async move {
            let requests = Vec::new();

            //process received requests

        });*/

        client
            .json_rpc_request(
                json_request(
                    1,
                    "state_call",
                    r#""Metadata_metadata_at_version", "0x0f000000""#,
                ),
                id,
            )
            .unwrap();

        let metadata: JsonResponse =
            serde_json::from_str(&responses.next().await.unwrap()).unwrap();

        let metadata = if let Value::String(hex_meta) = metadata.result {
            let b = unhex(&hex_meta).unwrap();
            let a = Option::<Vec<u8>>::decode(&mut &b[..]).unwrap();
            let meta = a.unwrap();
            if !meta.starts_with(&[109, 101, 116, 97]) {
                panic!("Rpc response error: metadata prefix 'meta' not found");
            };
            match RuntimeMetadata::decode(&mut &meta[4..]) {
                Ok(RuntimeMetadata::V15(out)) => out,
                Ok(_) => panic!("Only metadata V15 is supported"),
                Err(_) => panic!("Metadata could not be decoded"),
            }
        } else {
            panic!("wtf")
        };

        let req = json_request(
                    1,
                    "chain_getBlockHash",
                    r#"0"#,
                );
        println!("{}", req);
        client
            .json_rpc_request(
                req,
                id,
            )
            .unwrap();

        let res = &responses.next().await.unwrap();
        println!("{}", res);
        let genesis_hash: JsonResponse =
            serde_json::from_str(res).unwrap();

        let genesis_hash = if let Value::String(a) = genesis_hash.result {
            H256(unhex(&a).unwrap().try_into().unwrap())
        } else {
            panic!("block fetch failed")
        };

        client
            .json_rpc_request(json_request(1, "chain_getBlockHash", ""), id)
            .unwrap();

        let block_hash: JsonResponse =
            serde_json::from_str(&responses.next().await.unwrap()).unwrap();

        let block_hash = if let Value::String(a) = block_hash.result {
            H256(unhex(&a).unwrap().try_into().unwrap())
        } else {
            panic!("block fetch failed")
        };

        let req = json_request(1, "system_properties", "");//&format!("\"0x{}\"", hex::encode(block_hash.0)));
        println!("{}", req);
        client
            .json_rpc_request(req, id)
            .unwrap();

        let specs: Value =
            serde_json::from_str(&responses.next().await.unwrap()).unwrap();

        let specs = match &specs["result"] {
            Value::Object(a) => a.clone(),
            _ => panic!("specs is not a map: {:?}", specs),
        };

        client
            .json_rpc_request(json_request(1, "chain_getBlockHash", ""), id)
            .unwrap();

        let res = &responses.next().await.unwrap();
        println!("{}", res);
        let block_hash: JsonResponse =
            serde_json::from_str(res).unwrap();

        let block_hash = if let Value::String(a) = block_hash.result {
            H256(unhex(&a).unwrap().try_into().unwrap())
        } else {
            panic!("block fetch failed")
        };

        Self {
            block_hash,
            client,
            genesis_hash,
            id,
            responses,
            metadata,
            specs,
        }
    }

    pub fn metadata(&self) -> &RuntimeMetadataV15 {
        &self.metadata
    }

    pub fn genesis_hash(&self) -> H256 {
        self.genesis_hash
    }

    pub fn block(&self) -> H256 {
        self.block_hash
    }

    pub fn specs(&self) -> Map<String, Value> {
        self.specs.clone()
    }
}

const ADDRESS: &str = "wss://westend-rpc.polkadot.io";

/// Local errors
#[derive(Debug)]
enum Error {
    ChainCommunicationFailed,
    InvalidHex(String),
}

/// Generate JSON request from strings. Yes, like this. This is not dumber than imitating RPC
/// server inside app, so shut up. This works better and faster anyway.
fn json_request(index: u32, method: &str, params: &str) -> String {
    let part1 = r#"{"id":"#.to_owned();
    let part2 = r#","jsonrpc":"2.0","method":""#;
    let part3 = r#"","params":["#;
    let part4 = r#"]}"#;

    part1 + &format!("{}", index) + part2 + method + part3 + params + part4
}

#[derive(Deserialize)]
struct JsonResponse {
    id: usize,
    result: Value,
}

/// Strip "0x" prefix from input and parse it into numbers
fn unhex(hex_input: &str) -> Result<Vec<u8>, Error> {
    let hex_input_trimmed = {
        if let Some(hex_input_stripped) = hex_input.strip_prefix("0x") {
            hex_input_stripped
        } else {
            hex_input
        }
    };
    hex::decode(hex_input_trimmed).map_err(|_| Error::InvalidHex(hex_input.to_string()))
}

pub fn plop<T>(source: &mut broadcast::Receiver<T>) -> Option<T>
where
    T: Clone,
{
    match source.try_recv() {
        Ok(a) => return Some(a),
        Err(_) => {
            if let Ok(b) = source.try_recv() {
                return Some(b);
            }
        }
    };
    return None;
}

/*
pub async fn get_metadata(hash: H256) -> RuntimeMetadataV15 {
    let client = match WsClientBuilder::default()
        .build(ADDRESS.to_string()) //wss://node-shave.zymologia.fi:443".to_string())
        .await
    {
        Ok(a) => a,
        Err(e) => panic!("ws client builder crashed"),
    };

    let metadata: Value = client
        .request(
            "state_call",
            rpc_params!["Metadata_metadata_at_version", "0f000000"],
        )
        .await
        .unwrap();

    /* V14 legacy
    let metadata: Value = match client
        .request("state_getMetadata", rpc_params![hex::encode(hash.0)])
        .await
    {
        Ok(a) => a,
        Err(e) => panic!("{:?}", e),
    };
    */

    let metadata_v15 = if let Value::String(hex_meta) = metadata {
        let b = unhex(&hex_meta).unwrap();
        let a = Option::<Vec<u8>>::decode(&mut &b[..]).unwrap();
        let meta = a.unwrap();
        if !meta.starts_with(&[109, 101, 116, 97]) {
            panic!("Rpc response error: metadata prefix 'meta' not found");
        };
        match RuntimeMetadata::decode(&mut &meta[4..]) {
            Ok(RuntimeMetadata::V15(out)) => out,
            Ok(_) => panic!("Only metadata V15 is supported"),
            Err(_) => panic!("Metadata could not be decoded"),
        }
    } else {
        panic!("wtf")
    };
    metadata_v15
}

pub async fn get_genesis_hash() -> H256 {
    let client = match WsClientBuilder::default().build(ADDRESS.to_string()).await {
        Ok(a) => a,
        Err(e) => panic!("ws client builder crashed"),
    };
    let params = rpc_params![Value::Number(Number::from(0u8))];
    let genesis_hash_data: Value = match client.request("chain_getBlockHash", params).await {
        Ok(a) => a,
        Err(e) => {
            panic!("block fetch failed")
        }
    };
    if let Value::String(a) = genesis_hash_data {
        H256(unhex(&a).unwrap().try_into().unwrap())
    } else {
        panic!("block fetch failed")
    }
}

pub async fn get_specs(hash: H256) -> Map<String, Value> {
    let client = match WsClientBuilder::default().build(ADDRESS.to_string()).await {
        Ok(a) => a,
        Err(e) => panic!("ws client builder crashed"),
    };

    match client
        .request("system_properties", rpc_params![hex::encode(hash.0)])
        .await
    {
        Ok(a) => a,
        Err(e) => panic!("{:?}", e),
    }
}

pub fn block_watch() -> (broadcast::Receiver<H256>, broadcast::Receiver<H256>) {
    let (block_tx, mut block_rx) = broadcast::channel(1);
    let mut block_rx2 = block_tx.subscribe();

    tokio::spawn(async move {
        let client = match WsClientBuilder::default().build(ADDRESS.to_string()).await {
            Ok(a) => a,
            Err(e) => panic!("ws client builder crashed"),
        };
        loop {
            let params = rpc_params![];
            let block_hash_data: Value = match client.request("chain_getBlockHash", params).await {
                Ok(a) => a,
                Err(e) => {
                    panic!("block fetch failed")
                }
            };
            let block_hash = if let Value::String(a) = block_hash_data {
                a
            } else {
                panic!("block fetch failed")
            };
            block_tx
                .send(H256(unhex(&block_hash).unwrap().try_into().unwrap()))
                .unwrap();
            sleep(Duration::new(10, 0)).await;
        }
    });
    (block_rx, block_rx2)
}
*/

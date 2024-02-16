use frame_metadata::{v14::RuntimeMetadataV14, RuntimeMetadata};

use jsonrpsee::core::client::ClientT;
use jsonrpsee::{rpc_params, server::Server, ws_client::WsClientBuilder, RpcModule};

use parity_scale_codec::Decode;

use serde_json::{value::Value, Map, Number};

use tokio::{
    sync::{broadcast, mpsc},
    time::{sleep, Duration},
};

/// Local errors
#[derive(Debug)]
enum Error {
    ChainCommunicationFailed,
    InvalidHex(String),
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

pub async fn get_metadata(hash: &str) -> RuntimeMetadataV14 {
    let client = match WsClientBuilder::default()
        .build("wss://node-shave.zymologia.fi:443".to_string())
        .await
    {
        Ok(a) => a,
        Err(e) => panic!("ws client builder crashed"),
    };

    let metadata: Value = match client
        .request("state_getMetadata", rpc_params![&hash])
        .await
    {
        Ok(a) => a,
        Err(e) => panic!("{:?}", e),
    };

    let metadata_v14 = if let Value::String(hex_meta) = metadata {
        let meta = unhex(&hex_meta).unwrap();
        if !meta.starts_with(&[109, 101, 116, 97]) {
            panic!("Rpc response error: metadata prefix 'meta' not found");
        };
        match RuntimeMetadata::decode(&mut &meta[4..]) {
            Ok(RuntimeMetadata::V14(out)) => out,
            Ok(_) => panic!("Only metadata V14 is supported"),
            Err(_) => panic!("Metadata could not be decoded"),
        }
    } else {
        panic!("wtf")
    };
    metadata_v14
}

pub async fn get_genesis_hash() -> [u8; 32] {
    let client = match WsClientBuilder::default()
        .build("wss://node-shave.zymologia.fi:443".to_string())
        .await
    {
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
        unhex(&a).unwrap().try_into().unwrap()
    } else {
        panic!("block fetch failed")
    }
}

pub async fn get_specs(hash: &str) -> Map<String, Value> {
    let client = match WsClientBuilder::default()
        .build("wss://node-shave.zymologia.fi:443".to_string())
        .await
    {
        Ok(a) => a,
        Err(e) => panic!("ws client builder crashed"),
    };

    match client
        .request("system_properties", rpc_params![&hash])
        .await
    {
        Ok(a) => a,
        Err(e) => panic!("{:?}", e),
    }


}


pub fn block_watch() -> (broadcast::Receiver<String>, broadcast::Receiver<String>) {
    let (block_tx, mut block_rx) = broadcast::channel(1);
    let mut block_rx2 = block_tx.subscribe();

    tokio::spawn(async move {
        let client = match WsClientBuilder::default()
            .build("wss://node-shave.zymologia.fi:443".to_string())
            .await
        {
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
            block_tx.send(block_hash).unwrap();
            sleep(Duration::new(10, 0)).await;
        }
    });
    (block_rx, block_rx2)
}

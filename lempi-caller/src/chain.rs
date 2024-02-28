use frame_metadata::{v15::RuntimeMetadataV15, RuntimeMetadata};

use parity_scale_codec::Decode;

use primitive_types::H256;

use serde::Deserialize;
use serde_json::{value::Value, Map, Number};

use smoldot_light::{
    platform::DefaultPlatform, AddChainConfig, AddChainSuccess, ChainId, Client, JsonRpcResponses,
};

use std::{
    fs::File,
    io::{Read, Write},
    iter,
    num::NonZeroU32,
    sync::Arc,
};

use tokio::{
    macros::support::Poll,
    sync::{broadcast, mpsc},
    time::{sleep, Duration},
};

use crate::author::Address;

struct NonceRequest {
    id: H256,
    res: Option<u64>,
}

/// Abstraction to connect to chain
///
/// This should run asynchronously under the hood and provide easy synchronous observables
pub struct Blockchain {
    block_hash: H256,
    client: Client<Arc<DefaultPlatform>, ()>,
    genesis_hash: H256,
    id: ChainId,
    res: mpsc::Receiver<Value>,
    metadata: RuntimeMetadataV15,
    nonce_request: Option<NonceRequest>,
    specs: Map<String, Value>,
    log: Vec<String>,
}

impl Blockchain {
    pub async fn new(specpath: &str) -> Self {
        let mut client = Client::new(DefaultPlatform::new(
            env!("CARGO_PKG_NAME").into(),
            env!("CARGO_PKG_VERSION").into(),
        ));

        println!("{}", specpath);
        let mut spec = String::new();
        match File::open(specpath) {
            Ok(mut file) => file.read_to_string(&mut spec).unwrap(),
            Err(e) => panic!("{}", e),
        };
            
        let chain_config = AddChainConfig {
            user_data: (),
            specification: &spec,
            database_content: "",
            potential_relay_chains: iter::empty(),
            json_rpc: smoldot_light::AddChainConfigJsonRpc::Enabled {
                max_pending_requests: NonZeroU32::new(u32::max_value()).unwrap(),
                max_subscriptions: u32::max_value(),
            },
        };
        println!("smoldot started...");
        let AddChainSuccess {
            chain_id: id,
            json_rpc_responses: responses,
        } = client.add_chain(chain_config).unwrap();
        println!("chain connected...");
        let mut responses = responses.unwrap();

        client
            .json_rpc_request(json_request(1, "chain_getRuntimeVersion", ""), id)
            .unwrap();

        let version_r: JsonResponse =
            serde_json::from_str(&responses.next().await.unwrap()).unwrap();

        let version = if let Value::Number(a) = &version_r.result["specVersion"] {
            a.as_u64().unwrap()
        } else {
            panic!();
        };
        let name = if let Value::String(s) = &version_r.result["specName"] {
            s
        } else {
            panic!();
        };

        println!("{} version {}", name, version);

        let metadata_cache = metadata_cache(name, &version.to_string());

        let metadata = if let Ok(mut file) = File::open(&metadata_cache) {
            let mut hex_meta = String::new();
            file.read_to_string(&mut hex_meta).unwrap();
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

            let resp = &responses.next().await.unwrap();
            println!("{:?}", resp);
            let metadata_r: JsonResponse = serde_json::from_str(&responses.next().await.unwrap()).unwrap();

            if let Value::String(hex_meta) = metadata_r.result {
                let mut file = File::create(metadata_cache).unwrap();
                file.write_all(hex_meta.as_bytes()).unwrap();

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
            }
        };

        println!("metadata fetched...");

        let req = json_request(1, "chain_getBlockHash", r#"0"#);
        client.json_rpc_request(req, id).unwrap();

        let res = &responses.next().await.unwrap();
        let genesis_hash: JsonResponse = serde_json::from_str(res).unwrap();

        let genesis_hash = if let Value::String(a) = genesis_hash.result {
            H256(unhex(&a).unwrap().try_into().unwrap())
        } else {
            panic!("block fetch failed")
        };
        println!("genesis hash fetched...");

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
        println!("a block fetched...");

        let req = json_request(1, "system_properties", ""); //&format!("\"0x{}\"", hex::encode(block_hash.0)));
        client.json_rpc_request(req, id).unwrap();

        let specs: Value = serde_json::from_str(&responses.next().await.unwrap()).unwrap();

        let specs = match &specs["result"] {
            Value::Object(a) => a.clone(),
            _ => panic!("specs is not a map: {:?}", specs),
        };
        println!("specs fetched...");

        // Start block reception
        client
            .json_rpc_request(json_request(2, "chain_subscribeFinalizedHeads", ""), id)
            .unwrap();

        let _ = &responses.next().await.unwrap();

        let (rpc_tx, mut res) = mpsc::channel(256);

        tokio::spawn(async move {
            loop {
                let r: Value = serde_json::from_str(&responses.next().await.unwrap()).unwrap();
                rpc_tx.send(r).await.unwrap();
            }
        });

        Self {
            block_hash,
            client,
            genesis_hash,
            id,
            res,
            metadata,
            nonce_request: None,
            specs,
            log: Vec::new(),
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

    pub fn nonce(&mut self, address: H256, ss58: u16) -> Option<u64> {
        match &self.nonce_request {
            Some(a) => {
                if a.id == address {
                    a.res
                } else {
                    self.nonce_request = Some(NonceRequest {
                        id: address,
                        res: None,
                    });
                    let req = json_request(
                        2,
                        "system_accountNextIndex",
                        &format!(
                            "\"{}\"",
                            Address::from_public(address)
                                .into_account_id32()
                                .as_base58(ss58)
                                .to_string()
                        ),
                    );
                    self.client.json_rpc_request(req, self.id).unwrap();

                    None
                }
            }
            None => {
                self.nonce_request = Some(NonceRequest {
                    id: address,
                    res: None,
                });
                let req = json_request(
                    2,
                    "system_accountNextIndex",
                    &format!(
                        "\"{}\"",
                        Address::from_public(address)
                            .into_account_id32()
                            .as_base58(ss58)
                            .to_string()
                    ),
                );
                self.client.json_rpc_request(req, self.id).unwrap();

                None
            }
        }
    }

    pub fn send(&mut self, unchecked_extrinsic: &[u8]) {
        let req = json_request(
            9,
            "author_submitAndWatchExtrinsic",
            &format!("\"0x{}\"", hex::encode(&unchecked_extrinsic)),
        );
        self.log.push(format!("{}", req));
        self.client.json_rpc_request(req, self.id).unwrap();
    }

    pub fn log(&mut self) -> String {
        let mut out = String::new();
        while let Some(a) = self.log.pop() {
            out += "chain: ";
            out += &a;
            out += "\r\n";
        }
        out
    }

    pub fn crank(&mut self) -> bool {
        let mut modified = false;
        while let Ok(a) = self.res.try_recv() {
            modified = true;
            let mut unknown = true;
            match &a["method"] {
                Value::String(s) => match s.as_str() {
                    "chain_finalizedHead" => match &a["params"]["result"]["parentHash"] {
                        Value::String(h) => {
                            self.block_hash = H256(unhex(&h).unwrap().try_into().unwrap());
                            unknown = false;
                        }
                        _ => (),
                    },
                    _ => (),
                },
                _ => (),
            }
            match &a["id"].as_u64() {
                Some(2) => match &a["result"] {
                    Value::Number(n) => match self.nonce_request {
                        Some(ref mut b) => {
                            b.res = Some(n.as_u64().unwrap());
                            unknown = false;
                        }
                        None => (),
                    },
                    _ => (),
                },
                Some(9) => self.log.push(format!("submitted: {:?}", a)),
                _ => (),
            }
            if unknown {
                self.log.push(format!("Something else received: {:?}", a))
            };
        }
        modified
    }
}

fn metadata_cache(name: &str, version: &str) -> String {
    format!("../cache/metadata_{}_{}.tmp", name, version)
}

fn specs_cache(name: &str, version: &str) -> String {
    format!("../cache/specs_{}_{}.tmp", name, version)
}

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

#[derive(Debug, Deserialize)]
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

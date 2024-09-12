use frame_metadata::{v15::RuntimeMetadataV15, RuntimeMetadata};

use jsonrpsee::core::client::{ClientT, Subscription, SubscriptionClientT};
use jsonrpsee::rpc_params;
use jsonrpsee::ws_client::{WsClient, WsClientBuilder};

use parity_scale_codec::{Decode, DecodeAll};

use primitive_types::H256;

use serde::Deserialize;
use serde_json::{value::Value, Map, Number};

use substrate_constructor::{
    fill_prepare::{
        prepare_type, EraToFill, PrimitiveToFill, RegularPrimitiveToFill, SpecialTypeToFill,
        SpecialtyUnsignedToFill, TransactionToFill, TypeContentToFill, TypeToFill, UnsignedToFill,
        VariantSelector, DEFAULT_PERIOD,
    },
    finalize::Finalize,
    storage_query::{
        EntrySelector, EntrySelectorFunctional, FinalizedStorageQuery, StorageEntryTypeToFill,
        StorageSelector, StorageSelectorFunctional,
    },
};

use substrate_crypto_light::common::AsBase58;
use substrate_parser::{AsMetadata, ShortSpecs};
use substrate_parser::{
    cards::{ExtendedData, FieldData, ParsedData},
    decode_all_as_type,
    decoding_sci::Ty,
    propagated::Propagated,
    special_indicators::SpecialtyUnsignedInteger,
    ResolveType,
};

use std::{
    fs::File,
    future::Future,
    io::{Read, Write},
    iter,
    num::NonZeroU32,
    pin::Pin,
    sync::Arc,
};

use tokio::{
    macros::support::Poll,
    sync::{broadcast, mpsc},
    time::{sleep, Duration},
};

use crate::author::Address;

/// Abstraction to distinguish block hash from many other H256 things
#[derive(Debug, Clone)]
pub struct BlockHash(pub primitive_types::H256);

impl BlockHash {
    /// Convert block hash to RPC-friendly format
    pub fn to_string(&self) -> String {
        format!("0x{}", hex::encode(&self.0))
    }

    /// Convert string returned by RPC to typesafe block
    ///
    /// TODO: integrate nicely with serde
    pub fn from_str(s: &str) -> Self {
        let block_hash_raw = unhex(&s).unwrap();
        BlockHash(H256(
            block_hash_raw
                .try_into().unwrap(),
        ))
    }
}

struct NonceRequest {
    id: H256,
    res: tokio::sync::oneshot::Receiver<Value>,
    nonce: Option<u64>,
}

/// Fetch some runtime version identifier.
///
/// This does not have to be typesafe or anything; this could be used only to check if returned
/// value changes - and reboot the whole connection then, regardless of nature of change.
pub async fn runtime_version_identifier(
    client: &WsClient,
    block: &BlockHash,
) -> Value {
    client
        .request("state_getRuntimeVersion", rpc_params![block.to_string()])
        .await.unwrap()
}

pub async fn subscribe_blocks(client: &WsClient) -> Subscription<BlockHead> {
    client
        .subscribe(
            "chain_subscribeFinalizedHeads",
            rpc_params![],
            "chain_unsubscribeFinalizedHeads",
        )
        .await.unwrap()
}

pub async fn get_value_from_storage(
    client: &WsClient,
    whole_key: &str,
    block: &BlockHash,
) -> Value {
    client
        .request(
            "state_getStorage",
            rpc_params![whole_key, block.to_string()],
        )
        .await.unwrap()
}

/// Abstraction to connect to chain
///
/// This should run asynchronously under the hood and provide easy synchronous observables
pub struct Blockchain {
    block: BlockHash,
    block_number: u32,
    client: WsClient,
    genesis_hash: BlockHash,
    metadata: RuntimeMetadataV15,
    nonce_request: Option<NonceRequest>,
    extrinsic_watcher: Option<tokio::sync::mpsc::Receiver<Value>>,
    specs: ShortSpecs,
    log: Vec<String>,
}

impl Blockchain {
    pub async fn new(specpath: &str) -> Self {
        let client = WsClientBuilder::default().build(/*"wss://polkadot.api.onfinality.io/public-ws").await.unwrap();*/"wss://rpc.polkadot.io").await.unwrap();
        let genesis_hash = genesis_hash(&client).await;
        let mut blocks = subscribe_blocks(&client).await;
        let block = next_block(&client, &mut blocks).await;
        let version = runtime_version_identifier(&client, &block).await;
        let metadata = metadata(&client, &block).await;
        let block_number = current_block_number(&client, &metadata, &block).await;
        let name = <RuntimeMetadataV15 as AsMetadata<()>>::spec_name_version(&metadata).unwrap().spec_name;
        let specs = specs(&client, &metadata, &block).await;

    Self {
            block: block.clone(),
            block_number,
            client,
            genesis_hash,
            metadata,
            nonce_request: None,
            extrinsic_watcher: None,
            specs,
            log: vec![format!("Connected to {name} version {version} at block {block:?}")],
        }
    }

    pub fn metadata(&self) -> &RuntimeMetadataV15 {
        &self.metadata
    }

    pub fn genesis_hash(&self) -> H256 {
        self.genesis_hash.0
    }

    pub fn block(&self) -> H256 {
        self.block.0
    }

    pub fn block_number(&self) -> u32 {
        self.block_number
    }

    pub fn specs(&self) -> ShortSpecs {
        self.specs.clone()
    }

    pub async fn nonce(&mut self, address: H256) -> Option<u64> {
        if let Some(req) = &mut self.nonce_request {
            if req.id == address {
                req.nonce
            } else {
                let (tx, res) = tokio::sync::oneshot::channel();
                get_nonce(&self.client, &address.to_string(), tx);
                *req = NonceRequest {
                    id: address,
                    res,
                    nonce: None,
                };
                None
            }
        } else {
            let (tx, res) = tokio::sync::oneshot::channel();
            get_nonce(&self.client, &address.to_string(), tx);
            self.nonce_request = Some(
                NonceRequest {
                    id: address,
                    res,
                    nonce: None,
                }
            );
            None
        }
    }

    pub async fn send(&mut self, unchecked_extrinsic: &[u8]) {
        let rpc_params = rpc_params![format!("{}", hex::encode(unchecked_extrinsic))];
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        self.extrinsic_watcher = Some(rx);
        let client = &self.client;
        let response: Value = client
            .request("author_submitExtrinsic", rpc_params).await.unwrap();
        self.log.push(format!("extrinsic submitted: {}, response {response}", format!("0x{}", hex::encode(unchecked_extrinsic))));
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
        if let Some(nonce_request) = &mut self.nonce_request {
            if let Ok(a) = nonce_request.res.try_recv() {
                modified = true;
                nonce_request.nonce = Some(a.as_u64().unwrap());
            };
        }
        if let Some(extrinsic_watcher) = &mut self.extrinsic_watcher {
            if let Ok(a) = extrinsic_watcher.try_recv() {
                modified = true;
                self.log.push(format!("{a:?}"));
            }
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

/// fetch genesis hash, must be a hexadecimal string transformable into
/// H256 format
pub async fn genesis_hash(client: &WsClient) -> BlockHash {
    let genesis_hash_request: Value = client
        .request(
            "chain_getBlockHash",
            rpc_params![Value::Number(Number::from(0u8))],
        )
        .await
        .unwrap();
    match genesis_hash_request {
        Value::String(x) => BlockHash::from_str(&x),
        _ => panic!("ChainError::GenesisHashFormat"),
    }
}

/// fetch block hash, to request later the metadata and specs for
/// the same block
pub async fn block_hash(
    client: &WsClient,
    number: Option<String>,
) -> BlockHash {
    let rpc_params = if let Some(a) = number {
        rpc_params![a]
    } else {
        rpc_params![]
    };
    let block_hash_request: Value = client
        .request("chain_getBlockHash", rpc_params)
        .await.unwrap();
    match block_hash_request {
        Value::String(x) => BlockHash::from_str(&x),
        _ => panic!("block hash is not string")
    }
}

pub async fn current_block_number(
    client: &WsClient,
    metadata: &RuntimeMetadataV15,
    block: &BlockHash,
) -> u32 {
    let block_number_query = block_number_query(metadata);
    let fetched_value = get_value_from_storage(client, &block_number_query.key, block).await;
    if let Value::String(hex_data) = fetched_value {
        let value_data = unhex(&hex_data).unwrap();
        let value = decode_all_as_type::<&[u8], (), RuntimeMetadataV15>(
            &block_number_query.value_ty,
            &value_data.as_ref(),
            &mut (),
            &metadata.types,
        ).unwrap();
        if let ParsedData::PrimitiveU32 {
            value,
            specialty: _,
        } = value.data
        {
            value
        } else {
            panic!("ChainError::BlockNumberFormat")
        }
    } else {
        panic!("ChainError::StorageValueFormat(fetched_value)")
    }
}

/// fetch metadata at known block
pub async fn metadata(
    client: &WsClient,
    block: &BlockHash,
) -> RuntimeMetadataV15 {
    let metadata_request: Value = client
        .request(
            "state_call",
            rpc_params![
                "Metadata_metadata_at_version",
                "0x0f000000",
                block.to_string()
            ],
        )
        .await.unwrap();
    match metadata_request {
        Value::String(x) => {
            let metadata_request_raw = unhex(&x).unwrap();
            let maybe_metadata_raw = Option::<Vec<u8>>::decode_all(&mut &metadata_request_raw[..]).unwrap();
            if let Some(meta_v15_bytes) = maybe_metadata_raw {
                if meta_v15_bytes.starts_with(b"meta") {
                    match RuntimeMetadata::decode_all(&mut &meta_v15_bytes[4..]) {
                        Ok(RuntimeMetadata::V15(runtime_metadata_v15)) => {
                            return runtime_metadata_v15
                        }
                        Ok(_) => panic!("ChainError::NoMetadataV15"),
                        Err(_) => panic!("ChainError::MetadataNotDecodeable"),
                    }
                } else {
                    panic!("ChainError::NoMetaPrefix");
                }
            } else {
                panic!("ChainError::NoMetadataV15");
            }
        }
        _ => panic!("ChainError::MetadataFormat"),
    };
}

// fetch specs at known block
pub async fn specs(
    client: &WsClient,
    metadata: &RuntimeMetadataV15,
    block: &BlockHash,
) -> ShortSpecs {
    let specs_request: Value = client
        .request("system_properties", rpc_params![block.to_string()])
        .await.unwrap();
    match specs_request {
        Value::Object(properties) => system_properties_to_short_specs(&properties, &metadata),
        _ => panic!("ChainError::PropertiesFormat"),
    }
}

pub async fn next_block_number(blocks: &mut Subscription<BlockHead>) -> String {
    match blocks.next().await {
        Some(Ok(a)) => a.number,
        Some(Err(e)) => panic!("{}", e),
        None => panic!("ChainError::BlockSubscriptionTerminated"),
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct BlockHead {
    //digest: Value,
    //extrinsics_root: String,
    pub number: String,
    //parent_hash: String,
    //state_root: String,
}

pub async fn next_block(
    client: &WsClient,
    blocks: &mut Subscription<BlockHead>,
) -> BlockHash {
    block_hash(&client, Some(next_block_number(blocks).await)).await
}

pub fn system_properties_to_short_specs(
    system_properties: &Map<String, Value>,
    metadata: &RuntimeMetadataV15,
) -> ShortSpecs {
    let optional_prefix_from_meta = optional_prefix_from_meta(metadata);
    let base58prefix = base58prefix(system_properties, optional_prefix_from_meta);
    let decimals = decimals(system_properties);
    let unit = unit(system_properties);
    ShortSpecs {
        base58prefix,
        decimals,
        unit,
    }
}

pub fn base58prefix(
    x: &Map<String, Value>,
    optional_prefix_from_meta: Option<u16>,
) -> u16 {
    let base58prefix: u16 = match x.get("ss58Format") {
        // base58 prefix is fetched in `system_properties` rpc call
        Some(a) => match a {
            // base58 prefix value is a number
            Value::Number(b) => match b.as_u64() {
                // number is integer and could be represented as `u64` (the only
                // suitable interpretation available for `Number`)
                Some(c) => match c.try_into() {
                    // this `u64` fits into `u16` that base58 prefix is supposed
                    // to be
                    Ok(d) => match optional_prefix_from_meta {
                        // base58 prefix was found in `SS58Prefix` constant of
                        // the network metadata
                        //
                        // check that the prefixes match
                        Some(prefix_from_meta) => {
                            if prefix_from_meta == d {
                                d
                            } else {
                                panic!("aaa");/*
                                return Err(ChainError::Base58PrefixMismatch {
                                    specs: d,
                                    meta: prefix_from_meta,
                                });*/
                            }
                        }

                        // no base58 prefix was found in the network metadata
                        None => d,
                    },

                    // `u64` value does not fit into `u16` base58 prefix format,
                    // this is an error
                    Err(_) => {
                        panic!("(ChainError::Base58PrefixFormatNotSupported(a.to_string())")
                    }
                },

                // base58 prefix value could not be presented as `u64` number,
                // this is an error
                None => panic!("ChainError::Base58PrefixFormatNotSupported(a.to_string())"),
            },

            // base58 prefix value is not a number, this is an error
            _ => panic!("ChainError::Base58PrefixFormatNotSupported(a.to_string())"),
        },

        // no base58 prefix fetched in `system_properties` rpc call
        None => match optional_prefix_from_meta {
            // base58 prefix was found in `SS58Prefix` constant of the network
            // metadata
            Some(prefix_from_meta) => prefix_from_meta,

            // no base58 prefix at all, this is an error
            None => panic!("ChainError::NoBase58Prefix"),
        },
    };
    base58prefix
}

pub fn unit(x: &Map<String, Value>) -> String {
    match x.get("tokenSymbol") {
        // unit info is fetched in `system_properties` rpc call
        Some(a) => match a {
            // fetched unit value is a `String`
            Value::String(b) => {
                // definitive unit found
                b.to_string()
            }

            // fetched an array of units
            Value::Array(b) => {
                // array with a single element
                if b.len() == 1 {
                    // single `String` element array, process same as `String`
                    if let Value::String(c) = &b[0] {
                        // definitive unit found
                        c.to_string()
                    } else {
                        // element is not a `String`, this is an error
                        panic!("ChainError::UnitFormatNotSupported(a.to_string())")
                    }
                } else {
                    // units are an array with more than one element
                    panic!("ChainError::UnitFormatNotSupported(a.to_string())")
                }
            }

            // unexpected unit format
            _ => panic!("ChainError::UnitFormatNotSupported(a.to_string())"),
        },

        // unit missing
        None => panic!("ChainError::NoUnit"),
    }
}

pub fn decimals(x: &Map<String, Value>) -> u8 {
    match x.get("tokenDecimals") {
        // decimals info is fetched in `system_properties` rpc call
        Some(a) => match a {
            // fetched decimals value is a number
            Value::Number(b) => match b.as_u64() {
                // number is integer and could be represented as `u64` (the only
                // suitable interpretation available for `Number`)
                Some(c) => match c.try_into() {
                    // this `u64` fits into `u8` that decimals is supposed to be
                    Ok(d) => d,

                    // this `u64` does not fit into `u8`, this is an error
                    Err(_) => panic!("ChainError::DecimalsFormatNotSupported(a.to_string())"),
                },

                // number could not be represented as `u64`, this is an error
                None => panic!("ChainError::DecimalsFormatNotSupported(a.to_string())"),
            },

            // fetched decimals is an array
            Value::Array(b) => {
                // array with only one element
                if b.len() == 1 {
                    // this element is a number, process same as
                    // `Value::Number(_)`
                    if let Value::Number(c) = &b[0] {
                        match c.as_u64() {
                            // number is integer and could be represented as
                            // `u64` (the only suitable interpretation available
                            // for `Number`)
                            Some(d) => match d.try_into() {
                                // this `u64` fits into `u8` that decimals is
                                // supposed to be
                                Ok(f) => f,

                                // this `u64` does not fit into `u8`, this is an
                                // error
                                Err(_) => {
                                    panic!("ChainError::DecimalsFormatNotSupported(a.to_string())")
                                }
                            },

                            // number could not be represented as `u64`, this is
                            // an error
                            None => panic!("ChainError::DecimalsFormatNotSupported(a.to_string())"),
                        }
                    } else {
                        // element is not a number, this is an error
                        panic!("ChainError::DecimalsFormatNotSupported(a.to_string())")
                    }
                } else {
                    // decimals are an array with more than one element
                    panic!("ChainError::DecimalsFormatNotSupported(a.to_string())")
                }
            }

            // unexpected decimals format
            _ => panic!("ChainError::DecimalsFormatNotSupported(a.to_string())"),
        },

        // decimals are missing
        None => panic!("ChainError::NoDecimals"),
    }
}

pub fn optional_prefix_from_meta(metadata: &RuntimeMetadataV15) -> Option<u16> {
    let mut base58_prefix_data = None;
    for pallet in &metadata.pallets {
        if pallet.name == "System" {
            for system_constant in &pallet.constants {
                if system_constant.name == "SS58Prefix" {
                    base58_prefix_data = Some((&system_constant.value, &system_constant.ty));
                    break;
                }
            }
            break;
        }
    }
    if let Some((value, ty_symbol)) = base58_prefix_data {
        match decode_all_as_type::<&[u8], (), RuntimeMetadataV15>(
            ty_symbol,
            &value.as_ref(),
            &mut (),
            &metadata.types,
        ) {
            Ok(extended_data) => match extended_data.data {
                ParsedData::PrimitiveU8 {
                    value,
                    specialty: _,
                } => Some(value.into()),
                ParsedData::PrimitiveU16 {
                    value,
                    specialty: _,
                } => Some(value),
                ParsedData::PrimitiveU32 {
                    value,
                    specialty: _,
                } => value.try_into().ok(),
                ParsedData::PrimitiveU64 {
                    value,
                    specialty: _,
                } => value.try_into().ok(),
                ParsedData::PrimitiveU128 {
                    value,
                    specialty: _,
                } => value.try_into().ok(),
                _ => None,
            },
            Err(_) => None,
        }
    } else {
        None
    }
}

pub async fn get_nonce(
    client: &WsClient,
    account_id: &str,
    tx: tokio::sync::oneshot::Sender<Value>,
) {
    let rpc_params = rpc_params![account_id];
    let blah = client.request("account_nextIndex", rpc_params).await.unwrap();
    tokio::spawn(async move {
        //TODO lol this should probably build an own client or send to client manager thread
        //instead
        tx.send(blah);
    });
}

pub async fn send_stuff(client: &WsClient, data: &str) {
    let rpc_params = rpc_params![data];
    let mut subscription: Subscription<Value> = client
        .subscribe("author_submitAndWatchExtrinsic", rpc_params, "")
        .await.unwrap();
    let _reply = subscription.next().await.unwrap();
}

pub fn block_number_query(
    metadata_v15: &RuntimeMetadataV15,
) -> FinalizedStorageQuery {
    let storage_selector = StorageSelector::init(&mut (), metadata_v15).unwrap();

    if let StorageSelector::Functional(mut storage_selector_functional) = storage_selector {
        let mut index_system_in_pallet_selector = None;

        for (index, pallet) in storage_selector_functional
            .available_pallets
            .iter()
            .enumerate()
        {
            if pallet.prefix == "System" {
                index_system_in_pallet_selector = Some(index);
                break;
            }
        }

        if let Some(index_system_in_pallet_selector) = index_system_in_pallet_selector {
            // System - Number (current block number)
            storage_selector_functional =
                StorageSelectorFunctional::new_at::<(), RuntimeMetadataV15>(
                    &storage_selector_functional.available_pallets,
                    &mut (),
                    &metadata_v15.types,
                    index_system_in_pallet_selector,
                ).unwrap();

            if let EntrySelector::Functional(ref mut entry_selector_functional) =
                storage_selector_functional.query.entry_selector
            {
                let mut entry_index = None;
                for (index, entry) in entry_selector_functional
                    .available_entries
                    .iter()
                    .enumerate()
                {
                    if entry.name == "Number" {
                        entry_index = Some(index);
                        break;
                    }
                }
                if let Some(entry_index) = entry_index {
                    *entry_selector_functional =
                        EntrySelectorFunctional::new_at::<(), RuntimeMetadataV15>(
                            &entry_selector_functional.available_entries,
                            &mut (),
                            &metadata_v15.types,
                            entry_index,
                        ).unwrap();

                    storage_selector_functional
                        .query
                        .finalize()
                        .transpose().unwrap().unwrap()
                } else {
                    panic!("ChainError::NoBlockNumberDefinition")
                }
            } else {
                panic!("ChainError::NoStorageInSystem")
            }
        } else {
            panic!("ChainError::NoSystem")
        }
    } else {
        panic!("ChainError::NoStorage")
    }
}


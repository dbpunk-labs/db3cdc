//
// main.rs
// Copyright (C) 2022 db3.network Author imotai <codego.me@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//    http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
use shadow_rs::shadow;
shadow!(build);
use db3_base::{get_address_from_pk, strings};
use db3_crypto::signer::Db3Signer;
use db3_error::{DB3Error, Result};
use db3_proto::db3_base_proto::{ChainId, ChainRole};
use db3_proto::db3_mutation_proto::Mutation;
use db3_proto::db3_mutation_proto::{KvPair, MutationAction};
use db3_proto::db3_node_proto::storage_node_client::StorageNodeClient;
use db3_sdk::mutation_sdk::MutationSDK;
use db3_sdk::store_sdk::StoreSDK;
use db3cdc::event_key;
use db3cdc::gtid_state::GtidState;
use fastcrypto::traits::KeyPair;
use http::Uri;
use mysql_cdc::binlog_client::BinlogClient;
use mysql_cdc::binlog_options::BinlogOptions;
use mysql_cdc::constants::EVENT_HEADER_SIZE;
use mysql_cdc::events::event_parser::EventParser;
use mysql_cdc::events::event_type::EventType;
use mysql_cdc::replica_options::ReplicaOptions;
use mysql_cdc::ssl_mode::SslMode;
use std::sync::Arc;
use tonic::transport::{ClientTlsConfig, Endpoint};
use tracing::info;
use tracing_subscriber::filter::LevelFilter;
const ABOUT: &str = r"
(  _`\ (  _`\  /'_  )   (  _`\ (  _`\ (  _`\ 
| | ) || (_) )(_)_) |   | ( (_)| | ) || ( (_)
| | | )|  _ <' _(_ <    | |  _ | | | )| |  _
| |_) || (_) )( )_) |   | (_( )| |_) || (_( )
(____/'(____/'`\____)   (____/'(____/'(____/'
any issues are welcome
https://github.com/dbpunk-labs/db3cdc/issues
";
use clap::{Parser, Subcommand};
const GTID_KEY: [u8; 1] = [0];
#[derive(Debug, Parser)]
#[clap(name = "db3")]
#[clap(about = ABOUT, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Start a interactive shell
    #[clap()]
    Sync {
        /// the url of db3 node grpc api
        #[clap(long, default_value = "http://127.0.0.1:26659")]
        db3_node_grpc_url: String,
        #[clap(long, default_value = "mysql")]
        my_namespace: String,
        #[clap(long, default_value = "127.0.0.1")]
        master_host: String,
        #[clap(long, default_value = "3306")]
        master_port: u16,
        #[clap(long, default_value = "root")]
        user: String,
        #[clap(long)]
        password: String,
        #[clap(long, default_value = "true")]
        execlude_query_event: bool,
    },

    /// Get the version of DB3 CDC
    #[clap()]
    Version {},
}

async fn recover_gtid(
    client: Arc<StorageNodeClient<tonic::transport::Channel>>,
    ns: &str,
) -> Result<Option<GtidState>> {
    let kp = db3_cmd::get_key_pair(false).unwrap();
    let signer = Db3Signer::new(kp);
    let mut store_sdk = StoreSDK::new(client, signer);
    match store_sdk.open_session().await {
        Ok(r) => {
            let sid = r.session_id;
            if let Ok(Some(batch_get_values)) = store_sdk
                .batch_get(ns.as_bytes(), vec![GTID_KEY.to_vec()], sid)
                .await
            {
                store_sdk.close_session(sid).await.unwrap();
                if let Some(kv_pair) = batch_get_values.values.first() {
                    if let Ok(data) = std::str::from_utf8(kv_pair.value.as_ref()) {
                        info!("find step data {}", data);
                        let gstate_result: serde_json::Result<GtidState> =
                            serde_json::from_str(data);
                        match gstate_result {
                            Ok(gstate) => {
                                return Ok(Some(gstate));
                            }
                            Err(e) => {
                                return Err(DB3Error::QueryKvError(format!("{}", e)));
                            }
                        }
                    } else {
                        info!("fail to convert data to utf-8 str");
                    }
                } else {
                    info!("no gtid state in db3");
                }
            }
        }
        Err(e) => {
            info!("fail to open session for {}", e);
        }
    }
    Ok(None)
}

async fn start_sync(command: Commands) -> Result<()> {
    if let Commands::Sync {
        db3_node_grpc_url,
        my_namespace,
        master_host,
        master_port,
        user,
        password,
        execlude_query_event,
    } = command
    {
        tracing_subscriber::fmt()
            .with_max_level(LevelFilter::INFO)
            .init();
        let uri = db3_node_grpc_url.parse::<Uri>().unwrap();
        let endpoint = match uri.scheme_str() == Some("https") {
            true => {
                let rpc_endpoint = Endpoint::new(db3_node_grpc_url)
                    .unwrap()
                    .tls_config(ClientTlsConfig::new())
                    .unwrap();
                rpc_endpoint
            }
            false => {
                let rpc_endpoint = Endpoint::new(db3_node_grpc_url).unwrap();
                rpc_endpoint
            }
        };
        let channel = endpoint.connect_lazy();
        let grpc_client = Arc::new(StorageNodeClient::new(channel));
        let gstate = recover_gtid(grpc_client.clone(), my_namespace.as_str()).await;
        let kp = db3_cmd::get_key_pair(true).unwrap();
        let signer = Db3Signer::new(kp);
        let db3_sdk = MutationSDK::new(grpc_client.clone(), signer);
        let kp = db3_cmd::get_key_pair(false).unwrap();
        let addr = get_address_from_pk(&kp.public().pubkey);
        let signer = Db3Signer::new(kp);
        let store_sdk = StoreSDK::new(grpc_client.clone(), signer);
        let binlog_options = match gstate {
            Ok(Some(GtidState::MySQLState(gtid_set))) => BinlogOptions::from_mysql_gtid(gtid_set),
            Ok(Some(GtidState::MariaDB(gtid_list))) => BinlogOptions::from_mariadb_gtid(gtid_list),
            Ok(Some(GtidState::Position((filename, p)))) => {
                BinlogOptions::from_position(filename, p)
            }
            Ok(None) | Err(_) => BinlogOptions::from_start(),
        };
        info!("binlog options {:?}", binlog_options);
        let options = ReplicaOptions {
            hostname: master_host,
            port: master_port,
            username: user,
            password: password,
            blocking: true,
            ssl_mode: SslMode::Disabled,
            binlog: binlog_options,
            ..Default::default()
        };
        let mut binlog_client = BinlogClient::new(options);
        //TODO get it from db3
        let mut nonce = 1;
        if let Ok((events, checksum)) = binlog_client.replicate_raw() {
            let mut kvs: Vec<KvPair> = Vec::new();
            let header = KvPair {
                key: vec![0],
                value: vec![],
                action: MutationAction::InsertKv.into(),
            };
            kvs.push(header);
            let mut parser = EventParser::new();
            parser.checksum_type = checksum;
            for result in events {
                if let Ok((header, data)) = result {
                    info!(
                        "header timestamp {}, event_type {}, next_event_position {}",
                        header.timestamp, header.event_type, header.next_event_position
                    );
                    match EventType::from_code(header.event_type) {
                        EventType::HeartbeatEvent => {
                            continue;
                        }
                        EventType::RotateEvent => {
                            // handle binlog rotate
                            let event_slice = &data[1 + EVENT_HEADER_SIZE..];
                            // todo handle the error
                            let event = parser.parse_event(&header, event_slice).unwrap();
                            binlog_client.commit(&header, &event);
                            let position = GtidState::Position((
                                binlog_client.options.binlog.filename.to_string(),
                                binlog_client.options.binlog.position,
                            ));
                            let json_data = serde_json::to_string(&position).unwrap();
                            // TODO handle error
                            info!(
                                "rotate binlog {} {}",
                                binlog_client.options.binlog.filename.as_str(),
                                binlog_client.options.binlog.position
                            );
                            if let Some(header) = kvs.first_mut() {
                                header.value = json_data.as_bytes().to_vec();
                            }
                            continue;
                        }
                        EventType::QueryEvent
                        | EventType::MySqlGtidEvent
                        | EventType::XidEvent
                        | EventType::MariaDbGtidEvent => {
                            // update the step of synchronization
                            let event_slice = &data[1 + EVENT_HEADER_SIZE..];
                            // todo handle the error
                            let event = parser.parse_event(&header, event_slice).unwrap();
                            binlog_client.commit(&header, &event);
                            // update gtid for mysql
                            if let Some(ref mysql_gtidset) = binlog_client.options.binlog.gtid_set {
                                info!("update gtid state for mysql {:?}", mysql_gtidset);
                                let gstate = GtidState::MySQLState(mysql_gtidset.clone());
                                // TODO handle error
                                let json_data = serde_json::to_string(&gstate).unwrap();
                                if let Some(header) = kvs.first_mut() {
                                    header.value = json_data.as_bytes().to_vec();
                                }
                            }
                            if let Some(ref maridb_gtidlist) =
                                binlog_client.options.binlog.gtid_list
                            {
                                info!("update gtid state for maridb {:?}", maridb_gtidlist);
                                let gstate = GtidState::MariaDB(maridb_gtidlist.clone());
                                //TODO handle error
                                let json_data = serde_json::to_string(&gstate).unwrap();
                                if let Some(header) = kvs.first_mut() {
                                    header.value = json_data.as_bytes().to_vec();
                                }
                            }
                        }
                        _ => {}
                    };
                    if execlude_query_event {
                        if header.event_type == EventType::QueryEvent as u8 {
                            continue;
                        }
                    }
                    if let Ok(key) = event_key::encode_header(&header) {
                        let kv = KvPair {
                            key: key.to_owned(),
                            value: data.to_owned(),
                            action: MutationAction::InsertKv.into(),
                        };
                        kvs.push(kv);
                        let position = GtidState::Position((
                            binlog_client.options.binlog.filename.to_string(),
                            header.next_event_position,
                        ));
                        info!(
                            "binlog {} {}",
                            binlog_client.options.binlog.filename.as_str(),
                            binlog_client.options.binlog.position
                        );
                        // TODO handle error
                        let json_data = serde_json::to_string(&position).unwrap();
                        if let Some(header) = kvs.first_mut() {
                            header.value = json_data.as_bytes().to_vec();
                        }
                    }
                }
                nonce += 1;
                if kvs.len() >= 10 {
                    // the step key
                    let header = KvPair {
                        key: vec![0],
                        value: vec![],
                        action: MutationAction::InsertKv.into(),
                    };
                    let mutation = Mutation {
                        ns: my_namespace.as_bytes().to_vec(),
                        kv_pairs: kvs.drain(0..).collect(),
                        nonce: nonce,
                        chain_id: ChainId::MainNet.into(),
                        chain_role: ChainRole::StorageShardChain.into(),
                        gas_price: None,
                        gas: 10,
                    };
                    kvs.push(header);
                    if let Ok(r) = db3_sdk.submit_mutation(&mutation).await {
                        info!("mutation id {:?}", r);
                    }
                    if let Ok(a) = store_sdk.get_account(&addr).await {
                        let inner_account = a.clone();
                        let bills = inner_account.total_bills;
                        let credits = inner_account.credits;
                        info!("Your account {:?} status: total bills {}, total storage used {}, total mutation {}, credits {}",
                              addr,  strings::units_to_readable_num_str(&bills.unwrap()),
                                      strings::bytes_to_readable_num_str(a.total_storage_in_bytes),
                                    a.total_mutation_count,
                                    strings::units_to_readable_num_str(&credits.unwrap())
                              );
                    }
                }
            }
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    match args.command {
        Commands::Sync { .. } => start_sync(args.command).await.unwrap(),
        Commands::Version { .. } => {
            if shadow_rs::tag().len() > 0 {
                println!("version:{}", shadow_rs::tag());
            } else {
                println!(
                    "warning: a development version being used in branch {}",
                    shadow_rs::branch()
                );
            }
            println!("commit:{}", build::SHORT_COMMIT);
        }
    }
}

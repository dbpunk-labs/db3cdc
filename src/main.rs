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
use db3cdc::event_key;
use db3_crypto::signer::Db3Signer;
use db3_error::{DB3Error, Result};
use db3_proto::db3_base_proto::{ChainId, ChainRole};
use db3_proto::db3_mutation_proto::{KvPair, MutationAction};
use db3_proto::db3_node_proto::storage_node_client::StorageNodeClient;
use db3_sdk::mutation_sdk::MutationSDK;
use db3_proto::db3_mutation_proto::Mutation;
use std::sync::Arc;
use http::Uri;
use mysql_cdc::binlog_client::BinlogClient;
use mysql_cdc::binlog_options::BinlogOptions;
use mysql_cdc::errors::Error;
use mysql_cdc::replica_options::ReplicaOptions;
use mysql_cdc::ssl_mode::SslMode;
use mysql_cdc::events::event_type::EventType;
use tonic::transport::{ClientTlsConfig, Endpoint};
use tracing::{info, warn};
use tracing_subscriber::filter::LevelFilter;
const ABOUT: &str = r"
(  _`\ (  _`\  /'_  )   (  _`\ (  _`\ (  _`\ 
| | ) || (_) )(_)_) |   | ( (_)| | ) || ( (_)
| | | )|  _ <' _(_ <    | |  _ | | | )| |  _ 
| |_) || (_) )( )_) |   | (_( )| |_) || (_( )
(____/'(____/'`\____)   (____/'(____/'(____/'
";

use clap::{Parser, Subcommand};

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
        my_namespace:String,
        #[clap(long, default_value = "127.0.0.1")]
        master_host: String,
        #[clap(long, default_value = "3306")]
        master_port: u16,
        #[clap(long, default_value = "root")]
        user: String,
        #[clap(long)]
        password: String,
    },

    /// Get the version of DB3 CDC
    #[clap()]
    Version {},
}

async fn start_sync(command: Commands) -> Result<()> {
    if let Commands::Sync {
        db3_node_grpc_url,
        my_namespace,
        master_host,
        master_port,
        user,
        password,
    } = command
    {
        tracing_subscriber::fmt().with_max_level(LevelFilter::INFO).init();
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
        let kp = db3_cmd::get_key_pair(true).unwrap();
        let signer = Db3Signer::new(kp);
        let db3_sdk = MutationSDK::new(grpc_client, signer);
        let options = BinlogOptions::from_start();
        let options = ReplicaOptions {
            hostname: master_host,
            port: master_port,
            username: user,
            password: password,
            blocking: true,
            ssl_mode: SslMode::Disabled,
            binlog: options,
            ..Default::default()
        };
        let mut binlog_client = BinlogClient::new(options);
        //TODO get it from db3
        let mut nonce = 1;
        if let Ok(events) = binlog_client.replicate_raw() {
            let mut kvs: Vec<KvPair> = Vec::new();
            for result in events {
                if let Ok((header, data)) = result {
                    info!("header timestamp {}, event_type {}, next_event_position {}", header.timestamp, header.event_type,
                          header.next_event_position);
                    if header.event_type ==  EventType::RotateEvent as u8
                       || header.event_type == EventType::HeartbeatEvent as u8 {
                            continue;
                    }
                    if let Ok(key) = event_key::encode_header(&header) {
                        let kv = KvPair {
                            key: key.to_owned(),
                            value: data.to_owned(),
                            action: MutationAction::InsertKv.into(),
                        };
                        kvs.push(kv);
                    }
                }
                nonce += 1;
                if kvs.len() >= 10 {
                    let mutation = Mutation {
                        ns: my_namespace.as_bytes().to_vec(),
                        kv_pairs: kvs.drain(0..).collect(),
                        nonce: nonce,
                        chain_id: ChainId::MainNet.into(),
                        chain_role: ChainRole::StorageShardChain.into(),
                        gas_price: None,
                        gas: 10,
                    };
                    if let Ok(r) = db3_sdk.submit_mutation(&mutation).await {
                        println!("{:?}", r);
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

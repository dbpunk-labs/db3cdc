# DB3 CDC

the first decentralized live backup tool for mysql, why we need db3 cdc
* Many web3 developers use mysql to provide good experience to their users and try hard to build a decentralized data architecture to keep transparent with ther users
* No straight way to fully decentralized data architecture. let's approach it step by step.

So decentralized data backup is a good start to build a fully decentralized data architecture

`Warning`: Using DB3 and DB3 CDC in production enviroment is not recomended

# How it works

![how_it_works](./images/db3_cdc_how_it_works.svg)

# Getting Started

1. Download the release of db3 cdc from [github](https://github.com/dbpunk-labs/db3cdc/releases/tag/v0.1.0) according to your operation system, if you want run db3 cdc in linux 

```shell
wget https://github.com/dbpunk-labs/db3cdc/releases/download/v0.1.0/db3cdc-v0.1.0-linux-x86_64.tar.gz
tar -zxf db3cdc-v0.1.0-linux-x86_64.tar.gz
cd ./db3cdc-v0.1.0-linux-x86_64/
./bin/db3cdc --help
Usage: db3cdc <COMMAND>

Commands:
  sync     Start a interactive shell
  version  Get the version of DB3 CDC
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help information
```
2. use db3cdc to replicat mysql to db3

```
./bin/db3cdc sync --db3-node-grpc-url https://grpc.devnet.db3.network\
 --password mysql_password \
 --user mysql_root_user \
 --master-host mysql_host \
 --master-port mysql_port

```
You will see some information

```2022-11-19T02:54:26.886795Z  INFO db3cdc: find step data {"Position":["binlog.000003",2751]}
WARNING, db3 will generate private key and save it to ~/.db3/key
restore the key with addr 0x0dce49e41905e6c0c5091adcedee2dee524a3b06
2022-11-19T02:54:26.887920Z  INFO db3cdc: binlog options BinlogOptions { filename: "binlog.000003", position: 2751, gtid_set: None, gtid_list: None, starting_strategy: FromPosition }
2022-11-19T02:54:27.107724Z  INFO db3cdc: header timestamp 0, event_type 4, next_event_position 0
2022-11-19T02:54:27.107882Z  INFO db3cdc: rotate binlog binlog.000003 2751
2022-11-19T02:54:27.107964Z  INFO db3cdc: header timestamp 1668687954, event_type 15, next_event_position 0
2022-11-19T02:54:27.108027Z  INFO db3cdc: binlog binlog.000003 2751
2022-11-19T02:54:27.108108Z  INFO db3cdc: header timestamp 1668735474, event_type 30, next_event_position 2791
2022-11-19T02:54:27.108158Z  INFO db3cdc: binlog binlog.000003 2751
2022-11-19T02:54:27.108226Z  INFO db3cdc: header timestamp 1668735474, event_type 16, next_event_position 2822
2022-11-19T02:54:27.108282Z  INFO db3cdc: binlog binlog.000003 2822
2022-11-19T02:54:27.108357Z  INFO db3cdc: header timestamp 1668787201, event_type 4, next_event_position 2866
2022-11-19T02:54:27.108425Z  INFO db3cdc: rotate binlog binlog.000004 4
2022-11-19T02:54:27.108486Z  INFO db3cdc: header timestamp 0, event_type 4, next_event_position 0
2022-11-19T02:54:27.108549Z  INFO db3cdc: rotate binlog binlog.000004 4
2022-11-19T02:54:27.108609Z  INFO db3cdc: header timestamp 1668787201, event_type 15, next_event_position 126
2022-11-19T02:54:27.108664Z  INFO db3cdc: binlog binlog.000004 4
2022-11-19T02:54:27.108739Z  INFO db3cdc: header timestamp 1668787201, event_type 35, next_event_position 157
2022-11-19T02:54:27.108790Z  INFO db3cdc: binlog binlog.000004 4
```









//
// event_key.rs
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

use byteorder::{BigEndian, WriteBytesExt};
use db3_error::{DB3Error, Result};
use mysql_cdc::events::event_header::EventHeader;

pub fn encode_header(header: &EventHeader) -> Result<Vec<u8>> {
    let mut encoded_key: Vec<u8> = Vec::new();
    encoded_key
        .write_u32::<BigEndian>(header.timestamp)
        .map_err(|e| DB3Error::KeyCodecError(format!("{}", e)))?;
    encoded_key
        .write_u32::<BigEndian>(header.next_event_position)
        .map_err(|e| DB3Error::KeyCodecError(format!("{}", e)))?;
    encoded_key
        .write_u8(header.event_type)
        .map_err(|e| DB3Error::KeyCodecError(format!("{}", e)))?;
    Ok(encoded_key)
}

/*******************************************************************************
*   (c) 2020 Helium Systems, Inc
*
*  Licensed under the Apache License, Version 2.0 (the "License");
*  you may not use this file except in compliance with the License.
*  You may obtain a copy of the License at
*
*      http://www.apache.org/licenses/LICENSE-2.0
*
*  Unless required by applicable law or agreed to in writing, software
*  distributed under the License is distributed on an "AS IS" BASIS,
*  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
*  See the License for the specific language governing permissions and
*  limitations under the License.
********************************************************************************/

extern crate hidapi;
#[cfg(test)]
#[macro_use]
extern crate serial_test;

mod errors;
use byteorder::{BigEndian as BE, WriteBytesExt};
use errors::LedgerTcpError;
use ledger_apdu::{APDUAnswer, APDUCommand};
use std::result::Result;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;

pub struct TransportTcp {
    stream: TcpStream,
}

unsafe impl Sync for TransportTcp {}
unsafe impl Send for TransportTcp {}

impl TransportTcp {
    pub async fn new() -> Result<Self, errors::LedgerTcpError> {
        Ok(TransportTcp {
            stream: TcpStream::connect("127.0.0.1:9999")
                .await
                .map_err(|_| LedgerTcpError::connection_refused())?,
        })
    }
    pub async fn exchange(&mut self, command: &APDUCommand) -> Result<APDUAnswer, LedgerTcpError> {
        let payload = command.serialize();
        let mut data = Vec::new();
        WriteBytesExt::write_u32::<BE>(&mut data, payload.len() as u32)?;
        data.extend(payload);

        println!("{:?}", data);
        self.stream.write_all(&data).await?;
        // Wait for the socket to be readable
        self.stream.readable().await?;

        // Creating the buffer **after** the `await` prevents it from
        // being stored in the async task.
        let mut buf = [0; 512];

        // Try to read data, this may still fail with `WouldBlock`
        // if the readiness event is a false positive.
        match self.stream.try_read(&mut buf) {
            Ok(0) => Err(LedgerTcpError::connection_closed()),
            Ok(n) => Ok(APDUAnswer::from_answer(buf[..n].to_vec())),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                Err(LedgerTcpError::read_would_block())
            }
            Err(e) => Err(e.into()),
        }
    }
}

// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use snafu::ResultExt as _;

pub trait Value: Sized {
    fn encode_for_redis(&self) -> Result<Vec<u8>, super::Error>;
    fn decode_from_redis(raw: &[u8]) -> Result<Self, super::Error>;
}

impl<T: bincode::Encode + bincode::Decode<()> + serde::de::DeserializeOwned> Value for T {
    fn encode_for_redis(&self) -> Result<Vec<u8>, super::Error> {
        bincode::encode_to_vec(self, bincode::config::standard()).context(super::error::EncodeSnafu)
    }

    fn decode_from_redis(src: &[u8]) -> Result<Self, super::Error> {
        bincode::decode_from_slice(src, bincode::config::standard())
            .context(super::error::DecodeSnafu)
            .map(|(v, _)| v)
    }
}

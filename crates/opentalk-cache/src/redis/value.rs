// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

pub trait Value: bincode::Encode + bincode::Decode<()> + serde::de::DeserializeOwned {}

impl<T: bincode::Encode + bincode::Decode<()> + serde::de::DeserializeOwned> Value for T {}

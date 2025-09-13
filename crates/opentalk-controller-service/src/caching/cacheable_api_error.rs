// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use bincode::{BorrowDecode, Decode, Encode};
use http::StatusCode;
use opentalk_controller_utils::CaptureApiError;
use opentalk_types_api_v1::error::{ApiError, AuthenticationError, ErrorBody};
use serde::{Deserialize, Serialize};
use snafu::ResultExt as _;

use crate::Whatever;

/// (De)Serializable version of [`ApiError`] so it can be externally cached
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode)]
pub struct CacheableApiError {
    status: u16,
    www_authenticate: Option<AuthenticationError>,
    body: ErrorBody,
}

impl<C> Decode<C> for CacheableApiError {
    fn decode<D: bincode::de::Decoder<Context = C>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        Ok(Self {
            status: Decode::decode(decoder)?,
            www_authenticate: Decode::decode(decoder)?,
            body: Decode::decode(decoder)?,
        })
    }
}

impl<C> BorrowDecode<'static, C> for CacheableApiError {
    fn borrow_decode<D: bincode::de::BorrowDecoder<'static, Context = C>>(
        decoder: &mut D,
    ) -> Result<Self, bincode::error::DecodeError> {
        Ok(Self {
            status: Decode::decode(decoder)?,
            www_authenticate: Decode::decode(decoder)?,
            body: Decode::decode(decoder)?,
        })
    }
}

impl TryFrom<CacheableApiError> for ApiError {
    type Error = Whatever;

    fn try_from(value: CacheableApiError) -> Result<Self, Self::Error> {
        Ok(ApiError {
            status: StatusCode::from_u16(value.status).whatever_context("Invalid status code")?,
            www_authenticate: value.www_authenticate,
            body: value.body,
        })
    }
}

impl TryFrom<CacheableApiError> for CaptureApiError {
    type Error = Whatever;

    fn try_from(value: CacheableApiError) -> Result<Self, Self::Error> {
        Ok(ApiError::try_from(value)?.into())
    }
}

impl From<&ApiError> for CacheableApiError {
    fn from(value: &ApiError) -> Self {
        Self {
            status: value.status.as_u16(),
            www_authenticate: value.www_authenticate,
            body: value.body.clone(),
        }
    }
}

impl From<&CaptureApiError> for CacheableApiError {
    fn from(value: &CaptureApiError) -> Self {
        Self::from(&value.0)
    }
}

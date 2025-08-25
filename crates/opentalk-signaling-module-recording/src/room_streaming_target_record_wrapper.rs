// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use opentalk_inventory::RoomStreamingTargetRecord;
use opentalk_types_common::streaming::{StreamingKind, StreamingTargetKind};
use opentalk_types_signaling_recording::{StreamKindSecret, StreamStatus, StreamTargetSecret};
use snafu::{ResultExt as _, Snafu};

#[derive(Debug, Snafu)]
#[snafu(display("Parsing the {url_kind} url failed, because {target_url} is not a valid URL"))]
pub(super) struct StreamTargetConversionWrongUrlError {
    source: url::ParseError,
    target_url: String,
    url_kind: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, derive_more::From)]
pub(super) struct RoomStreamingTargetRecordWrapper(RoomStreamingTargetRecord);

impl TryFrom<RoomStreamingTargetRecordWrapper> for StreamTargetSecret {
    type Error = StreamTargetConversionWrongUrlError;

    fn try_from(
        RoomStreamingTargetRecordWrapper(value): RoomStreamingTargetRecordWrapper,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            kind: StreamKindSecret::Livestream(match value.kind {
                StreamingKind::Custom => StreamingTargetKind::Custom {
                    streaming_endpoint: value.streaming_endpoint.parse().context(
                        StreamTargetConversionWrongUrlSnafu {
                            target_url: value.streaming_endpoint,
                            url_kind: "streaming_endpoint",
                        },
                    )?,
                    streaming_key: value.streaming_key,
                    public_url: value.public_url.parse().context(
                        StreamTargetConversionWrongUrlSnafu {
                            target_url: value.public_url,
                            url_kind: "public_url",
                        },
                    )?,
                },
            }),
            status: StreamStatus::Inactive,
        })
    }
}

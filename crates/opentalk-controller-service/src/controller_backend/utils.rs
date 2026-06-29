// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::{cmp::Ordering, pin::Pin};

use async_stream::{__private::AsyncStream, stream};
use futures_util::{Stream, StreamExt};
use opentalk_controller_utils::{CaptureApiError, TariffResourceExt};
use opentalk_inventory::{Result, Room, utils::is_room_guest_access_allowed};
use opentalk_types_api_v1::error::ApiError;
use opentalk_types_common::{
    features::GUESTS_ALLOWED_MODULE_FEATURE_ID, rooms::GuestAccess, tariffs::TariffResource,
};

pub(crate) fn interweave_result_streams<'a, T: 'a>(
    streams: Vec<Pin<Box<dyn Stream<Item = Result<T>> + 'a>>>,
    item_comparer: fn(&T, &T) -> Ordering,
) -> AsyncStream<Result<T>, impl Future<Output = ()>> {
    let num_of_streams = streams.len();

    let mut streams = streams
        .into_iter()
        .map(|s| Box::pin(s.peekable()))
        .collect::<Vec<_>>();

    stream! {
        loop {
            // For each stream, peek for the next pending item
            let mut peeked_items = Vec::with_capacity(num_of_streams);
            for stream in &mut streams {
                peeked_items.push(stream.as_mut().peek().await);
            }

            // Determine the stream index of the smallest pending item from all pending items peeked above
            let smallest_item_stream_index = peeked_items.iter().enumerate().min_by(|(_index_1, item_1), (_index_2, item_2)| {
                match (item_1, item_2) {
                    (Some(item_1), Some(item_2)) => {
                        if let (Ok(item_1), Ok(item_2)) = (item_1, item_2) {
                            item_comparer(item_1, item_2)
                        } else {Ordering::Equal}
                    }
                    (Some(_item), None) => {Ordering::Less}
                    (None, Some(_item)) => {Ordering::Greater}
                    (None, None) => {Ordering::Equal}
                }
            })
            .map(|(index, _item)| index);

            // Return immediately if no smallest item could be determined (i.e. the streams vec is empty)
            let smallest_item_stream_index = match smallest_item_stream_index {
                Some(smallest_item_stream_index) => {smallest_item_stream_index}
                None => {return;}
            };

            // Fetch the smallest pending item from its stream and yield it
            if let Some(next_item) = streams[smallest_item_stream_index].next().await {
                yield next_item;
            } else { return; }
        }
    }
}

/// Verifies if invites can be read for a given room
/// Returns an error if invites the action is not allowed
pub fn verify_invite_read(tariff: &TariffResource, room: &Room) -> Result<(), CaptureApiError> {
    if is_room_guest_access_allowed(room, tariff) {
        Ok(())
    } else {
        Err(ApiError::not_found()
            .with_code("service_unavailable")
            .with_message("Invites are not available: either the guest feature is disabled, guest access is disabled for this room or the room is encrypted".to_string())
            .into())
    }
}

/// Verifies if invites can be written for a given room
/// Returns an error if invites the action is not allowed
pub fn verify_invite_write(tariff: &TariffResource, room: &Room) -> Result<(), CaptureApiError> {
    if is_room_guest_access_allowed(room, tariff) {
        Ok(())
    } else {
        Err(ApiError::forbidden()
            .with_code("service_unavailable")
            .with_message("Invites are not available: either the guest feature is disabled, guest access is disabled for this room or the room is encrypted".to_string())
            .into())
    }
}

pub fn ensure_guest_access_valid(
    guest_access: GuestAccess,
    e2ee_enabled: bool,
    tariff: &TariffResource,
) -> Result<(), ApiError> {
    if guest_access.is_disabled() {
        return Ok(());
    }

    if e2ee_enabled {
        return Err(ApiError::bad_request()
            .with_code("invalid_guest_access")
            .with_message("Guest access cannot be enabled for encrypted rooms".to_string()));
    }

    tariff.require_feature(&GUESTS_ALLOWED_MODULE_FEATURE_ID)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use http::StatusCode;
    use opentalk_types_common::{
        features::GUESTS_ALLOWED_MODULE_FEATURE_ID,
        rooms::GuestAccess,
        tariffs::{TariffId, TariffModuleResource, TariffResource},
    };

    use super::ensure_guest_access_valid;

    fn guests_allowed_tariff() -> TariffResource {
        TariffResource {
            id: TariffId::nil(),
            name: "Guest feature enabled".to_owned(),
            quotas: BTreeMap::new(),
            modules: BTreeMap::from_iter([(
                GUESTS_ALLOWED_MODULE_FEATURE_ID.module,
                TariffModuleResource {
                    features: BTreeSet::from([GUESTS_ALLOWED_MODULE_FEATURE_ID.feature]),
                },
            )]),
        }
    }

    fn guests_disabled_tariff() -> TariffResource {
        TariffResource {
            id: TariffId::nil(),
            name: "Guests feature disabled".to_owned(),
            quotas: BTreeMap::new(),
            modules: BTreeMap::new(),
        }
    }

    #[test]
    fn disabled_guest_access_is_always_valid() {
        // No feature checks apply when guest access is disabled
        ensure_guest_access_valid(GuestAccess::Disabled, true, &guests_disabled_tariff()).unwrap();
        ensure_guest_access_valid(GuestAccess::Disabled, false, &guests_disabled_tariff()).unwrap();
    }

    #[test]
    fn enabled_guest_access_is_valid_when_allowed_and_not_encrypted() {
        ensure_guest_access_valid(GuestAccess::DirectAccess, false, &guests_allowed_tariff())
            .unwrap();
        ensure_guest_access_valid(GuestAccess::WaitingRoom, false, &guests_allowed_tariff())
            .unwrap();
    }

    #[test]
    fn enabled_guest_access_rejected_when_e2ee_enabled() {
        let err =
            ensure_guest_access_valid(GuestAccess::DirectAccess, true, &guests_allowed_tariff())
                .unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST);
        assert_eq!(err.body.code, "invalid_guest_access");
    }

    #[test]
    fn enabled_guest_access_rejected_when_feature_disabled() {
        let err =
            ensure_guest_access_valid(GuestAccess::DirectAccess, false, &guests_disabled_tariff())
                .unwrap_err();
        assert_eq!(err.status, StatusCode::FORBIDDEN);
        assert_eq!(err.body.code, "feature_disabled");
    }
}

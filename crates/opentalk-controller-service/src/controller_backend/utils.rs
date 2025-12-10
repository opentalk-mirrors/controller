// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::cmp::Ordering;

use async_stream::{__private::AsyncStream, stream};
use futures_util::{Stream, StreamExt, pin_mut};
use opentalk_inventory::Result;

pub(crate) fn interweave_result_streams<'a, T: 'a>(
    stream_1: impl Stream<Item = Result<T>> + 'a,
    stream_2: impl Stream<Item = Result<T>> + 'a,
    item_comparer: fn(&T, &T) -> Ordering,
) -> AsyncStream<Result<T>, impl Future<Output = ()>> {
    stream! {
        // The next items to be processed. Used to pass leftovers from one iteration to the next.
        let mut pending_next_1 = None;
        let mut pending_next_2 = None;

        pin_mut!(stream_1);
        pin_mut!(stream_2);
        loop {
            // Fetch new items unless there are any leftovers from the previous iteration.
            if pending_next_1.is_none() { pending_next_1 = Some(stream_1.next().await)}
            if pending_next_2.is_none() { pending_next_2 = Some(stream_2.next().await)}

            let next_1 = pending_next_1.take().flatten();
            let next_2 = pending_next_2.take().flatten();

            match (next_1, next_2) {
                (Some(next_1), Some(next_2)) => {
                    let prioritize_item_2 = if let (Ok(next_1), Ok(next_2)) = (&next_1, &next_2) {
                        item_comparer(next_1, next_2) == Ordering::Greater
                    } else { false };

                    if prioritize_item_2 {
                        yield next_2;
                        pending_next_1 = Some(Some(next_1));
                    } else {
                        yield next_1;
                        pending_next_2 = Some(Some(next_2));
                    }
                }
                (Some(next_1), None) => {
                    yield next_1;
                }
                (None, Some(next_2)) => {
                    yield next_2;
                }
                (None, None) => {
                    break;
                }
            }
        }
    }
}

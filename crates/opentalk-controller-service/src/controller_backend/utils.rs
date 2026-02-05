// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::{cmp::Ordering, pin::Pin};

use async_stream::{__private::AsyncStream, stream};
use futures_util::{Stream, StreamExt};
use opentalk_inventory::Result;

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

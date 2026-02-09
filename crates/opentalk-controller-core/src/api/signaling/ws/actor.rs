// SPDX-FileCopyrightText: OpenTalk GmbH <mail@opentalk.eu>
//
// SPDX-License-Identifier: EUPL-1.2

use std::{
    convert::TryFrom,
    time::{Duration, Instant},
};

use actix::{Actor, ActorContext, AsyncContext, Handler, StreamHandler};
use actix_http::ws::{CloseCode, CloseReason, Item, ProtocolError};
use actix_web_actors::ws::{Message, WebsocketContext};
use bytes::BytesMut;
use bytestring::ByteString;
use opentalk_controller_settings::WebSocketRateLimit;
use snafu::Report;
use tokio::sync::mpsc::UnboundedSender;

use super::RunnerMessage;

/// The rate at which the configured rate limit token amount is added to the token bucket
const RATE_LIMIT_INTERVAL: Duration = Duration::from_secs(1);

/// Define HTTP Websocket actor
///
/// This actor will relay all text and binary received websocket messages to the given unbounded sender
/// It is up to the receiver of the channel to extract the underlying message.
///
/// Handling timeouts is also done in this actor.
pub struct WebSocketActor {
    /// Sender to signaling runner
    sender: UnboundedSender<RunnerMessage>,

    rate_limit: Option<RateLimit>,

    /// Timestamp of last pong received
    last_pong: Instant,

    /// State for receiving fragmented messages
    continuation: Option<Continuation>,

    close_sent: bool,
}

struct RateLimit {
    config: WebSocketRateLimit,

    bucket: u16,
}

impl RateLimit {
    fn new(config: WebSocketRateLimit) -> Self {
        let bucket = config.token_bucket_size;

        Self { config, bucket }
    }

    /// Adds the configured amount of tokens to the bucket, but no more than the maximum that is specified in the config.
    fn add_tokens(&mut self) {
        self.bucket = std::cmp::min(
            self.bucket + self.config.tokens_per_second,
            self.config.token_bucket_size,
        );
    }

    /// Consumes a token from the bucket
    ///
    /// Returns false if the bucket is empty
    fn consume_token(&mut self) -> bool {
        if self.bucket == 0 {
            return false;
        }

        self.bucket -= 1;

        true
    }
}

struct Continuation {
    buffer: BytesMut,
    is_text: bool,
}

impl WebSocketActor {
    pub fn new(
        sender: UnboundedSender<RunnerMessage>,
        rate_limit_config: Option<WebSocketRateLimit>,
    ) -> Self {
        let rate_limit = rate_limit_config.map(RateLimit::new);

        Self {
            sender,
            rate_limit,
            last_pong: Instant::now(),
            continuation: None,
            close_sent: false,
        }
    }

    /// Send a [`RunnerMessage`] to the runner
    ///
    /// Closes the websocket if the channel to the runner is disconnected and the socket hasn't been closed yet
    fn forward_to_runner(
        &mut self,
        ctx: &mut WebsocketContext<Self>,
        runner_message: RunnerMessage,
    ) {
        if self.sender.send(runner_message).is_err() && !self.close_sent {
            self.close_sent = true;
            ctx.close(Some(CloseReason {
                code: CloseCode::Abnormal,
                description: Some("runner disconnected".to_owned()),
            }));
        }
    }

    /// Handle continuation packages by saving them in a separate buffer
    fn handle_continuation(&mut self, ctx: &mut WebsocketContext<Self>, item: Item) {
        match item {
            Item::FirstText(bytes) => {
                if self.continuation.is_some() {
                    log::warn!("Got continuation while processing one");
                }

                self.continuation = Some(Continuation {
                    buffer: BytesMut::from(&bytes[..]),
                    is_text: true,
                });
            }
            Item::FirstBinary(bytes) => {
                if self.continuation.is_some() {
                    log::warn!("Got continuation while processing one");
                }

                self.continuation = Some(Continuation {
                    buffer: BytesMut::from(&bytes[..]),
                    is_text: false,
                });
            }
            Item::Continue(bytes) => {
                if let Some(continuation) = &mut self.continuation {
                    continuation.buffer.extend_from_slice(&bytes);

                    if continuation.buffer.len() >= 1_000_000 {
                        log::error!("Fragmented message over 1 MB, stopping actor");
                        ctx.stop();
                    }
                } else {
                    log::warn!("Got continuation continue message without a continuation set");
                }
            }
            Item::Last(bytes) => {
                if let Some(mut continuation) = self.continuation.take() {
                    continuation.buffer.extend_from_slice(&bytes);

                    if continuation.is_text {
                        match ByteString::try_from(continuation.buffer) {
                            Ok(string) => {
                                self.forward_to_runner(ctx, Message::Text(string).into());
                            }
                            Err(_) => {
                                log::warn!(
                                    "Got text continuation item but it wasn't valid UTF8, discarding"
                                );
                            }
                        }
                    } else {
                        self.forward_to_runner(
                            ctx,
                            Message::Binary(continuation.buffer.freeze()).into(),
                        );
                    }
                } else {
                    log::warn!("Got continuation last message without a continuation set");
                }
            }
        }
    }
}

impl Actor for WebSocketActor {
    type Context = WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        let sender = self.sender.clone();

        // Start an interval for connection checks via ping-pong
        ctx.run_interval(Duration::from_secs(15), move |this, ctx| {
            if Instant::now().duration_since(this.last_pong) > Duration::from_secs(20) {
                // no response to ping, exit
                ctx.stop();
                // in this case we don't really need to take care of the error
                let _ = sender.send(RunnerMessage::Timeout);
            } else {
                ctx.ping(b"heartbeat");
            }
        });

        if self.rate_limit.is_some() {
            ctx.run_interval(RATE_LIMIT_INTERVAL, move |this, _ctx| {
                if let Some(rate_limit) = &mut this.rate_limit {
                    rate_limit.add_tokens();
                }
            });
        }
    }
}

/// Handle incoming websocket messages
impl StreamHandler<Result<Message, ProtocolError>> for WebSocketActor {
    fn handle(&mut self, msg: Result<Message, ProtocolError>, ctx: &mut Self::Context) {
        if let Some(rate_limit) = &mut self.rate_limit
            && !rate_limit.consume_token()
        {
            log::trace!("Rate limit reached");
            self.forward_to_runner(ctx, RunnerMessage::RateLimitReached);
        }

        match msg {
            Ok(Message::Ping(msg)) => ctx.pong(&msg),
            Ok(Message::Pong(msg)) => {
                if msg == b"heartbeat"[..] {
                    self.last_pong = Instant::now();
                }
            }
            Ok(msg @ Message::Text(_)) => {
                self.forward_to_runner(ctx, msg.into());
            }
            Ok(msg @ Message::Binary(_)) => {
                self.forward_to_runner(ctx, msg.into());
            }
            Ok(Message::Continuation(item)) => self.handle_continuation(ctx, item),
            Ok(msg @ Message::Close(_)) => {
                // Pass the Close frame to the runner to handle
                self.forward_to_runner(ctx, msg.into());
            }
            Ok(Message::Nop) => {}
            Err(e) => {
                log::warn!(
                    "Protocol error in websocket - exiting, {}",
                    Report::from_error(e)
                );

                ctx.stop();
            }
        }
    }
}

/// Command for the WebSocketActor sent by the runner
#[derive(actix::Message)]
#[rtype(result = "()")]
pub enum WsCommand {
    Ws(Message),
    Close(CloseReason),
}

/// Handle websocket messages produced by the runner, to be sent to the client
impl Handler<WsCommand> for WebSocketActor {
    type Result = ();

    fn handle(&mut self, msg: WsCommand, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            WsCommand::Ws(msg) => ctx.write_raw(msg),
            WsCommand::Close(reason) => {
                self.close_sent = true;
                ctx.close(Some(reason))
            }
        }
    }
}

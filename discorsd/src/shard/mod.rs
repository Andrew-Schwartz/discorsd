//! Functionality that manages a Bot's connection to Discord and receives events.

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Arc;

use async_tungstenite::{
    tokio::{connect_async, ConnectStream},
    tungstenite::Message,
    tungstenite::protocol::CloseFrame,
    tungstenite::protocol::frame::coding::CloseCode,
    WebSocketStream,
};
use futures::{SinkExt, TryStreamExt};
use itertools::Itertools;
use log::{error, info, warn};
use rand::Rng;
use thiserror::Error;
use tokio::sync::RwLockWriteGuard;
use tokio::time::{Duration, Instant};

use dispatch::DispatchPayload;
use model::{HelloPayload, Payload, Resume};

use crate::Bot;
use crate::bot::BotState;
use crate::cache::Update;
use crate::http::ClientError;
use crate::macros::API_VERSION;
use crate::model::command::ApplicationCommand;
use crate::model::ids::{CommandId, Id};
use crate::shard::model::Heartbeat;

pub mod model;
pub mod dispatch;
pub mod intents;
pub mod send;

pub type ShardResult<T> = Result<T, ShardError>;
pub type WsStream = WebSocketStream<ConnectStream>;
type WsError = async_tungstenite::tungstenite::Error;

// todo prune useless errors (esp Other)
#[derive(Debug, Error)]
pub enum ShardError {
    #[error("http error: {0}")]
    Request(#[from] ClientError),
    #[error("websocket error: {0}")]
    Websocket(#[from] WsError),
    #[error("stream closed (restarting)")]
    NeedRestart,
    #[error("stream closed (resuming)")]
    NeedResume,
    #[error("other error: {0}")]
    Other(String),
}

// internal types used by `Shard<B>`
/// [Shard::send](Shard::send) can either fail because of a websocket error or because
/// [Shard::stream](Shard::stream) is `None` (ie, need to restart the websocket)
pub(crate) enum SendError {
    Websocket(WsError),
    NeedRestart,
}

impl From<SendError> for ShardError {
    fn from(se: SendError) -> Self {
        match se {
            SendError::Websocket(wse) => Self::Websocket(wse),
            SendError::NeedRestart => Self::NeedRestart,
        }
    }
}

impl From<WsError> for SendError {
    fn from(wse: WsError) -> Self {
        Self::Websocket(wse)
    }
}

/// used by `Shard::events_loop` which can only ever return with an error
pub enum Never {}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum ConnectionAction {
    None,
    Resume,
    Reconnect,
}

impl ConnectionAction {
    fn terminal(self) -> bool {
        match self {
            Self::None => false,
            Self::Resume | Self::Reconnect => true,
        }
    }
}

fn gateway_params(url: &str) -> String {
    format!("{url}/?v={API_VERSION}&encoding=json")
}

pub struct Shard<B: Bot + 'static> {
    pub shard_info: (u64, u64),
    state: Arc<BotState<B>>,
    session_id: Option<String>,
    gateway: Option<String>,
    resume_gateway: Option<String>,
    seq: Option<u64>,
    heartbeat_interval: Option<Duration>,
    heartbeat: Option<Instant>,
    ack: Option<Instant>,
    strikes: u8,
}

impl<B: Bot + 'static> Shard<B> {
    pub fn new(state: Arc<BotState<B>>) -> Self {
        // let stream = Shard::connect(&state).await?;
        Self {
            shard_info: (0, 0),
            state,
            session_id: None,
            gateway: None,
            resume_gateway: None,
            seq: None,
            heartbeat_interval: None,
            heartbeat: None,
            ack: None,
            strikes: 0,
        }
    }

    async fn close<D: Into<Option<Duration>> + Send>(&mut self, close_frame: CloseFrame<'_>, delay: D) {
        // do this first so we don't hold it across the `.await`
        info!("closing: {:?}", close_frame);
        if let Some(mut stream) = self.state.stream.write().await.take() {
            if let Err(e) = stream.close(Some(close_frame)).await {
                error!("{}", e);
            }
            info!("stream closed");
        } else {
            info!("stream was already closed");
        }
        if let Some(delay) = delay.into() {
            info!("delaying for {:?}", delay);
            tokio::time::sleep(delay).await;
        }
    }

    pub async fn run(&mut self) -> Never {
        loop {
            let error = self._run().await;
            if let Err(e) = error {
                error!("Shard::_run error {}, restarting...", e);
            }
        }
    }

    async fn _run(&mut self) -> ShardResult<()> {
        // need to (re)connect
        if self.state.stream.read().await.is_none() {
            if self.gateway.is_none() {
                let url = self.state.client.gateway_bot()
                    .await?
                    .url;
                let ws = gateway_params(&url);
                self.gateway = Some(ws);
            }
            let ws = self.gateway.as_deref().unwrap();
            info!("connecting to {}", ws);
            let (stream, _): (WsStream, _) = connect_async(ws).await?;
            *self.state.stream.write().await = Some(stream);
        }

        if let (Some(session), &Some(seq)) = (&self.session_id, &self.seq) {
            let resume = Resume {
                token: self.state.client.token.clone(),
                session_id: session.clone(),
                seq,
            };
            println!("resume (in _run) = {resume:?}");
            send(&mut self.state.stream.write().await, resume).await?;
        }

        let action = self.events_loop().await;
        match &action {
            Err(ShardError::Request(_)) => {}
            Err(ShardError::Websocket(_)) => {
                // as far as I can tell, all websocket errors are fatal
                *self.state.stream.write().await = None;
            }
            Err(ShardError::NeedRestart) => {
                *self.state.stream.write().await = None;
            }
            Err(ShardError::NeedResume) => { todo!() }
            Err(ShardError::Other(_)) => {}
            Ok(ConnectionAction::None) => unreachable!(),
            Ok(ConnectionAction::Resume) => {
                println!("RESUMING!");
                self.close(CloseFrame {
                    code: CloseCode::Restart,
                    reason: "Initiating resume".into(),
                }, None).await;
                let Some(resume_gateway) = &self.resume_gateway else {
                    todo!("handle this?")
                };
                println!("resuming ({resume_gateway:?})");
                let (stream, _): (WsStream, _) = connect_async(resume_gateway).await?;
                *self.state.stream.write().await = Some(stream);
            }
            Ok(ConnectionAction::Reconnect) => {
                *self.state.stream.write().await = None;
            }
        }
        Ok(())
    }

    async fn events_loop(&mut self) -> ShardResult<ConnectionAction> {
        loop {
            if self.state.stream.read().await.is_none() {
                warn!("start of 'events loop with a None stream");
                return Err(ShardError::NeedRestart);
            };

            let action = self.heartbeat().await?;
            if action.terminal() { return Ok(action); }

            let result = tokio::time::timeout(
                Duration::from_millis(200),
                self.state.stream.write().await.as_mut().ok_or(ShardError::NeedRestart)?.try_next(),
            ).await;
            if let Ok(next) = result {
                match next {
                    Ok(Some(Message::Text(text))) => {
                        // let read = nice_from_str(&text);
                        let read = serde_json::from_str(&text);
                        let payload = match read {
                            Ok(payload) => payload,
                            Err(payload_parse_error) => {
                                error!("payload_parse_error = {}", payload_parse_error);
                                println!("{text}");
                                continue;
                            }
                        };
                        let action = self.handle_payload(payload).await?;
                        if action.terminal() { return Ok(action); }
                    }
                    Ok(Some(Message::Close(close_frame))) => {
                        error!("close frame = {:?}", close_frame);
                        self.reset_connection_state();
                        // todo handle resuming/reconnecting https://discord.com/developers/docs/topics/opcodes-and-status-codes#gateway-gateway-close-event-codes
                        self.close(close_frame.unwrap_or_else(|| CloseFrame {
                            code: CloseCode::Restart,
                            reason: "Received `Message::Close` (without a CloseFrame)".into(),
                        }), None).await;
                        return Err(ShardError::NeedRestart);
                    }
                    Ok(Some(msg)) => warn!("msg = {:?}", msg),
                    Ok(None) => {
                        error!("Websocket closed");
                        self.reset_connection_state();
                        self.close(CloseFrame {
                            code: CloseCode::Restart,
                            reason: "Websocket closed".into(),
                        }, None).await;
                        return Err(ShardError::NeedRestart);
                    }
                    Err(ws_error) => {
                        // Protocol("Connection reset without closing handshake")
                        // Io(Os { code: 104, kind: ConnectionReset, message: "Connection reset by peer" })
                        error!("ws_error = {:?}", ws_error);
                        self.reset_connection_state();
                        self.close(CloseFrame {
                            code: CloseCode::Error,
                            reason: "Websocket error".into(),
                        }, None).await;
                        return Err(ShardError::NeedRestart);
                    }
                }
            }
        }
    }

    async fn heartbeat(&mut self) -> Result<ConnectionAction, SendError> {
        if let (Some(heartbeat), Some(ack)) = (self.heartbeat, self.ack) {
            // If a client does not receive a heartbeat ACK between its attempts at sending
            // heartbeats, this may be due to a failed or "zombied" connection. The client should
            // immediately terminate the connection with any close code besides `1000` (Normal) or
            // `1001` (Away), then reconnect and attempt to Resume.
            if heartbeat > ack {
                println!("self.strikes = {:?}", self.strikes);
                // self.reset_connection_state();
                self.heartbeat = None;
                self.ack = None;
                self.close(CloseFrame {
                    code: CloseCode::Restart,
                    reason: "ACK not recent enough, closing websocket".into(),
                }, None).await;
                return Ok(ConnectionAction::Resume);
            }
        }

        match (self.heartbeat, self.heartbeat_interval, self.seq) {
            (Some(last_sent), Some(interval), _) if last_sent.elapsed() < interval => {}
            (_, _, Some(seq_num)) => {
                send(&mut self.state.stream.write().await, Heartbeat { seq_num }).await?;
                self.heartbeat = Some(Instant::now());
            }
            _ => {}
        }

        Ok(ConnectionAction::None)
    }

    /// handles `payload`, returns if we need to reconnect
    async fn handle_payload(&mut self, payload: Payload) -> ShardResult<ConnectionAction> {
        let need_reconnect = match payload {
            Payload::Hello(HelloPayload { heartbeat_interval }) => {
                if self.session_id.is_none() {
                    self.initialize_connection(heartbeat_interval).await?;
                }
                ConnectionAction::None
            }
            Payload::Dispatch { event, seq_num } => {
                if let Some(curr) = self.seq {
                    if seq_num > curr + 1 {
                        warn!("received seq num {}, expected {} ({} were missed)",
                              seq_num, curr + 1, seq_num - curr - 1
                        );
                    }
                }
                self.seq = Some(seq_num);
                self.handle_dispatch(event).await;
                ConnectionAction::None
            }
            Payload::HeartbeatAck => {
                self.ack = Some(Instant::now());
                ConnectionAction::None
            }
            Payload::Heartbeat(Heartbeat { seq_num }) => {
                info!("recv: Heartbeat {}", seq_num);
                send(&mut self.state.stream.write().await, Heartbeat { seq_num }).await?;
                self.heartbeat = Some(Instant::now());
                ConnectionAction::None
            }
            Payload::Reconnect => {
                info!("recv: Reconnect");
                self.close(CloseFrame {
                    code: CloseCode::Restart,
                    reason: "Reconnect requested by Discord".into(),
                }, None).await;
                ConnectionAction::Resume
            }
            Payload::InvalidSession(resumable) => {
                info!("recv: Invalid Session");
                if resumable {
                    warn!("Resumable Invalid Session: anything special to do here?");
                    ConnectionAction::Resume
                } else {
                    self.reset_connection_state();

                    let delay = rand::thread_rng().gen_range(1..=5);
                    self.close(CloseFrame {
                        code: CloseCode::Restart,
                        reason: "(non-resumable) Invalid Session".into(),
                    }, Duration::from_secs(delay)).await;
                    ConnectionAction::Reconnect
                }
            }
            _ => {
                error!("Should not receive {:?}", payload);
                ConnectionAction::None
            }
        };
        Ok(need_reconnect)
    }

    async fn initialize_connection(&mut self, heartbeat_interval: u64) -> Result<(), SendError> {
        let delay = Duration::from_millis(heartbeat_interval);
        self.heartbeat_interval = Some(delay);

        if self.session_id.is_none() {
            send(&mut self.state.stream.write().await, self.state.bot.identify()).await?;
        }

        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    async fn handle_dispatch(&mut self, event: DispatchPayload) /*-> ShardResult<()>*/ {
        use DispatchPayload::*;
        event.clone().update(&self.state.cache).await;
        if let Ready(ready) = &event {
            // make sure were using the right API version
            assert_eq!(API_VERSION, ready.v);

            // make sure we're the right shard
            let (id, tot) = ready.shard.unwrap_or((0, 0));
            assert_eq!(id, self.shard_info.0);
            assert_eq!(tot, self.shard_info.1);

            self.session_id = Some(ready.session_id.clone());
            let resume = gateway_params(&ready.resume_gateway_url);
            self.resume_gateway = Some(resume);

            if self.state.global_slash_commands.get().is_none() {
                fn set_commands<C: ?Sized>(
                    app_commands: &mut Vec<ApplicationCommand>,
                    commands: &[&'static C],
                ) -> HashMap<CommandId, &'static C> {
                    app_commands.drain(..commands.len())
                        .zip_eq(commands)
                        .map(|(ac, c)| (ac.id, *c))
                        .collect()
                }

                let app = ready.application.id;
                let client = &self.state.client;

                let slash_commands = B::global_commands();
                let user_commands = B::global_user_commands();
                let message_commands = B::global_message_commands();
                let global_commands = slash_commands
                    .iter().map(|c| c.command())
                    .chain(user_commands.iter().map(|c| c.command()))
                    .chain(message_commands.iter().map(|c| c.command()))
                    .collect();
                let mut commands = client
                    .bulk_overwrite_global_commands(
                        app,
                        global_commands,
                    ).await
                    .unwrap();
                self.state.global_slash_commands.get_or_init(|| set_commands(&mut commands, slash_commands));
                self.state.global_user_commands.get_or_init(|| set_commands(&mut commands, user_commands));
                self.state.global_message_commands.get_or_init(|| set_commands(&mut commands, message_commands));
            }
        }
        let state = Arc::clone(&self.state);
        // todo panic if this panicked? (make a field in self for handlers, try_join them?)
        let _handle = tokio::spawn(async move {
            let result = match event {
                Ready(_ready) => state.bot.ready(Arc::clone(&state)).await,
                Resumed(_resumed) => state.bot.resumed(Arc::clone(&state)).await,
                GuildCreate(guild) => {
                    // need to initialize some extra stuff that isn't sent in the gateway
                    // tokio::spawn({
                    //     let state = Arc::clone(&state);
                    //     let channels = guild.guild.channels.clone();
                    //     let name = guild.guild.name.clone();
                    //     let id = guild.guild.id;
                    //     async move {
                    for channel in &guild.guild.channels {
                        match state.client.get_channel(channel.id()).await {
                            Ok(channel) => dispatch::ChannelCreate { channel }.update(&state.cache).await,
                            Err(error) => error!(
                                "Error getting channel after GuildCreate({}): {}",
                                guild.guild.name.as_ref().unwrap_or(&guild.guild.id.to_string()),
                                error.display_error(&state).await,
                            ),
                        }
                    }
                    // }
                    // });
                    state.bot.guild_create(guild.guild, Arc::clone(&state)).await
                }
                MessageCreate(message) => state.bot.message_create(
                    message.message, Arc::clone(&state),
                ).await,
                MessageUpdate(update) => state.bot.message_update(
                    state.cache.message(update.id).await.unwrap(),
                    Arc::clone(&state),
                    update,
                ).await,
                InteractionCreate(dispatch::InteractionCreate { interaction }) => {
                    state.bot.interaction(
                        interaction, Arc::clone(&state),
                    )
                }.await,
                MessageReactionAdd(add) => state.bot.reaction(
                    add.into(),
                    Arc::clone(&state),
                ).await,
                MessageReactionRemove(remove) => state.bot.reaction(
                    remove.into(),
                    Arc::clone(&state),
                ).await,
                IntegrationUpdate(integration) => state.bot.integration_update(
                    integration.guild_id,
                    integration.integration,
                    Arc::clone(&state),
                ).await,
                GuildRoleCreate(create) => state.bot.role_create(
                    create.guild_id,
                    create.role,
                    Arc::clone(&state),
                ).await,
                GuildRoleUpdate(update) => state.bot.role_update(
                    update.guild_id,
                    update.role,
                    Arc::clone(&state),
                ).await,
                _ => Ok(())
            };
            if let Err(error) = result {
                state.bot.error(error, Arc::clone(&state)).await;
            }
        });

        // Ok(())
    }

    fn reset_connection_state(&mut self) {
        let Self {
            // stream,
            session_id,
            seq,
            heartbeat_interval,
            heartbeat,
            ack,
            strikes,
            // online,
            ..
        } = self;
        // *stream = None;
        *session_id = None;
        *seq = None;
        *heartbeat_interval = None;
        *heartbeat = None;
        *ack = None;
        *strikes = 0;
        // *online = false;
    }
}

pub(crate) async fn send<P>(stream: &mut RwLockWriteGuard<'_, Option<WsStream>>, payload: P) -> Result<(), SendError>
    where P: Into<Payload> + Display + Send
{
    info!("sending {}", payload);
    let message = serde_json::to_string(&payload.into())
        .expect("Payload serialization can't fail");
    stream
        .as_mut()
        .ok_or(SendError::NeedRestart)?
        .send(Message::Text(message)).await?;
    Ok(())
}
//! General infrastructure for using `discorsd`, including the [`Bot`](Bot) trait, which handles
//! events your bot receives, and the [`BotState`](BotState) struct, which stores your bot's state
//! and can be accessed in most of [`Bot`](Bot)'s methods.
//!
//! Many functions pass around or take as a parameter `Arc<BotState<B>>`, where `B` is the type of
//! your Bot. Other functions will be generic over a type parameter named `State` where
//! `State: AsRef<BotState<B>>`. This allows you to pass a `&state` to such functions, no matter if
//! the `state` you have is a `BotState<B>`, a `&BotState<B>`, or an `Arc<BotState<B>>`.
//!
//! Similarly, many functions take generic `Client` parameter where `Client: AsRef<DiscordClient>`.
//! This allows you to pass a reference to any of the above state types, or a reference to a
//! `DiscordClient` or `&DiscordClient`.

use std::collections::HashMap;
use std::fmt::{self, Debug};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use log::error;
use once_cell::sync::OnceCell;
use tokio::sync::{RwLock, RwLockWriteGuard};

use crate::cache::Cache;
use crate::commands::{ButtonCommand, MenuCommandRaw, ReactionCommand, SlashCommand, SlashCommandRaw, UserCommand, MessageCommand};
use crate::errors::BotError;
use crate::http::{ClientResult, DiscordClient};
use crate::model::commands::{InteractionUse, AppCommandData};
use crate::model::components::{Button, ComponentId, Menu, SelectMenuType};
use crate::model::guild::{Guild, Integration};
use crate::model::ids::*;
use crate::model::message::Message;
use crate::model::interaction;
use crate::model::interaction::{ApplicationCommandData, MessageComponentData};
use crate::model::permissions::Role;
use crate::model::user::User;
use crate::shard;
use crate::shard::dispatch::{MessageUpdate, ReactionUpdate};
use crate::shard::model::Identify;
use crate::shard::Shard;

/// Maps `GuildId` to a `RwLock<V>`.
pub type GuildIdMap<V> = HashMap<GuildId, RwLock<V>>;
/// Maps `CommandId` to a `SlashCommand`.
pub type GuildCommands<B> = HashMap<CommandId, Box<dyn SlashCommandRaw<Bot=B>>>;

/// Stores the state of your Bot.
pub struct BotState<B: Send + Sync + 'static> {
    /// The client, including your bot's token.
    pub client: DiscordClient,
    /// All information received in events.
    /// Also updated by `BotState::cache_SOMETHING`, which is otherwise the same as
    /// `DiscordClient::get_SOMETHING`.
    pub cache: Cache,
    /// Your bot type, storing whatever other data you need.
    pub bot: B,
    /// The [`SlashCommand`](SlashCommand)s your bot has created, mapped by guild.
    pub commands: RwLock<GuildIdMap<GuildCommands<B>>>,
    /// The [`SlashCommand`](SlashCommand) ids your bot has created, by name in each guild.
    pub command_names: RwLock<GuildIdMap<HashMap<&'static str, CommandId>>>,
    /// The global [`SlashCommand`](SlashCommand)s your bot has created.
    pub global_commands: OnceCell<HashMap<CommandId, &'static dyn SlashCommandRaw<Bot=B>>>,
    /// The global [`UserCommand`](UserCommand)s your bot has created.
    pub global_user_commands: OnceCell<HashMap<CommandId, &'static dyn UserCommand<Bot=B>>>,
    /// The global [`MessageCommand`](MessageCommand)s your bot has created.
    pub global_message_commands: OnceCell<HashMap<CommandId, &'static dyn MessageCommand<Bot=B>>>,
    /// The global [`SlashCommand`](SlashCommand) ids your bot has created, by name.
    pub global_command_names: OnceCell<HashMap<&'static str, CommandId>>,
    /// The global [`UserCommand`](UserCommand) ids your bot has created, by name.
    pub global_user_command_names: OnceCell<HashMap<&'static str, CommandId>>,
    /// The global [`MessageCommand`](MessageCommand) ids your bot has created, by name.
    pub global_message_command_names: OnceCell<HashMap<&'static str, CommandId>>,
    /// The [`ReactionCommand`](ReactionCommand)s your bot is using.
    pub reaction_commands: RwLock<Vec<Box<dyn ReactionCommand<B>>>>,
    pub buttons: std::sync::RwLock<HashMap<ComponentId, Box<dyn ButtonCommand<Bot=B>>>>,
    pub menus: std::sync::RwLock<HashMap<ComponentId, Box<dyn MenuCommandRaw<Bot=B>>>>,
    // todo need to also have a way to distinguish between separate bot runs, like the first
    //  interaction will always be 0 so you could use the old button or w/e and the new one would
    //  trigger
    pub count: AtomicUsize,
}

impl<B: Send + Sync + 'static> BotState<B> {
    fn create_id(&self) -> ComponentId {
        let id = self.count.fetch_add(1, Ordering::Relaxed);
        id.to_string().into()
    }

    pub(crate) fn register_button(&self, button: &mut Button, command: Box<dyn ButtonCommand<Bot=B>>) {
        let id = self.create_id();
        button.custom_id = Some(id.clone());
        self.buttons.write().unwrap().insert(id, command);
    }

    pub(crate) fn register_menu<T: SelectMenuType>(&self, menu: &mut Menu<T>, command: Box<dyn MenuCommandRaw<Bot=B>>) {
        let id = self.create_id();
        menu.custom_id = id.clone();
        self.menus.write().unwrap().insert(id, command);
    }

    pub async fn register_guild_commands<G: Id<Id=GuildId>, I: IntoIterator<Item=Box<dyn SlashCommandRaw<Bot=B>>>>(
        &self,
        guild: G,
        commands: I,
    ) -> ClientResult<()> {
        let guild = guild.id();
        for command in commands {
            let application_command = self.client.create_guild_command(
                self.cache.application_id(),
                guild,
                command.command(),
            ).await?;
            let name = command.name();
            self.commands.write()
                .await
                .entry(guild)
                .or_default()
                .write()
                .await
                .insert(application_command.id, command);
            self.command_names.write()
                .await
                .entry(guild)
                .or_default()
                .write()
                .await
                .insert(name, application_command.id);
        }
        Ok(())
    }
}

impl<B: Send + Sync> AsRef<Self> for BotState<B> {
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<B: Send + Sync> BotState<B> {
    // todo
    // #[cfg(test)]
    pub fn testing_state(bot: B) -> Arc<Self> {
        Arc::new(Self {
            client: DiscordClient::single(String::new()),
            cache: Default::default(),
            bot,
            commands: Default::default(),
            command_names: Default::default(),
            global_commands: Default::default(),
            global_user_commands: Default::default(),
            global_message_commands: Default::default(),
            global_command_names: Default::default(),
            global_user_command_names: Default::default(),
            global_message_command_names: Default::default(),
            reaction_commands: Default::default(),
            buttons: Default::default(),
            menus: Default::default(),
            count: Default::default(),
        })
    }

    /// Gets the current [`User`](User).
    ///
    /// # Panics
    ///
    /// If somehow used before [`Ready`](crate::shard::dispatch::Ready) is received.
    pub async fn user(&self) -> User {
        self.cache.own_user().await
    }

    /// Gets the bot's `ApplicationId`.
    ///
    /// # Panics
    ///
    /// If somehow used before [`Ready`](crate::shard::dispatch::Ready) is received.
    pub fn application_id(&self) -> ApplicationId {
        self.cache.application
            .get()
            .expect("")
            .id
    }

    /// Get the id of command `C` in this `guild`.
    ///
    /// # Note
    ///
    /// Locks [BotState::command_names](BotState::command_names) in read mode, meaning this can
    /// cause deadlocks if called while a write guard is held.
    pub async fn try_command_id<C: SlashCommand<Bot=B>>(&self, guild: GuildId) -> Option<CommandId> {
        self.command_names.read().await
            .get(&guild)?
            .read().await
            .get(C::NAME)
            .copied()
    }

    /// Get the id of command `C` in this `guild`.
    ///
    /// # Note
    ///
    /// Locks [BotState::command_names](BotState::command_names) in read mode, meaning this can
    /// cause deadlocks if called while a write guard is held.
    ///
    /// # Panics
    ///
    /// Panics if the bot is not in this `guild`, or if the command `C` does not exist
    /// in this guild.
    pub async fn command_id<C: SlashCommand<Bot=B>>(&self, guild: GuildId) -> CommandId {
        *self.command_names.read().await
            .get(&guild)
            .unwrap_or_else(|| panic!("Guild {} exists", guild))
            .read().await
            .get(C::NAME)
            .unwrap_or_else(|| panic!("{} exists", C::NAME))
    }

    /// Get the id of the global command `C`.
    ///
    /// # Note
    ///
    /// Locks [BotState::global_command_names](BotState::global_command_names) in read mode, meaning
    /// this can cause deadlocks if called while a write guard is held.
    ///
    /// # Panics
    ///
    /// Panics if the bot has not received the [Ready](crate::shard::dispatch::Ready) event yet, or if the
    /// command `C` does not exist is not a global command.
    pub async fn global_command_id<C: SlashCommand<Bot=B>>(&self) -> CommandId {
        *self.global_command_names.get()
            .expect("Bot hasn't connected yet")
            .get(C::NAME)
            .unwrap_or_else(|| panic!("{} exists", C::NAME))
    }
    // todo are global_user_command_id and global_message_command_id needed?

    // bots can't use these
    // /// Edits the [`default_permission`](crate::commands::Command::default_permission) to be true
    // /// for command `C` in this `guild`, meaning that everyone in the guild will be able to use it.
    // ///
    // /// # Panics
    // ///
    // /// Panics if the bot is not in this `guild`, or if the command `C` does not exist.
    // /// in this guild.
    // pub async fn enable_command<C: SlashCommand<Bot=B>>(&self, guild: GuildId) -> ClientResult<ApplicationCommand> {
    //     self.command_id::<C>(guild).await
    //         .default_permissions(self, guild, true).await
    // }
    //
    //
    // /// Edits the [`default_permission`](crate::commands::Command::default_permission) to be true
    // /// for command `C` in this `guild`, meaning that no one in the guild will be able to use it
    // /// unless the command's permissions were edited to allow their [`UserId`](UserId) or a
    // /// [`RoleId`] they have.
    // ///
    // /// # Panics
    // ///
    // /// Panics if the bot is not in this `guild`, or if the command `C` does not exist.
    // /// in this guild.
    // pub async fn disable_command<C: SlashCommand<Bot=B>>(&self, guild: GuildId) -> ClientResult<ApplicationCommand> {
    //     self.command_id::<C>(guild).await
    //         .default_permissions(self, guild, false).await
    // }

    /// Get a mutable [`SlashCommand`](SlashCommand) `C` by type.
    ///
    /// A mutable reference to a [`RwLockWriteGuard`](RwLockWriteGuard) must be passed in, which the
    /// lifetime of the returned mutable reference is tied to.
    ///
    /// # Panics
    ///
    /// Panics if the bot is not in this `guild`, or if the command `C` does not exist.
    #[allow(clippy::needless_lifetimes)]
    pub async fn get_command_mut<'c, C: SlashCommand<Bot=B>>(
        &self,
        guild: GuildId,
        // not ideal that it has to take this instead of just the guild.
        commands: &'c mut RwLockWriteGuard<'_, GuildCommands<B>>,
    ) -> (CommandId, &'c mut C) {
        let id = self.command_id::<C>(guild).await;
        commands.get_mut(&id)
            .and_then(|c| c.downcast_mut())
            .map(|command| (id, command))
            .unwrap_or_else(|| panic!("`{}` command exists", C::NAME))
    }
}

impl<B: Debug + Send + Sync> Debug for BotState<B> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BotState")
            .field("client", &self.client)
            .field("cache", &self.cache)
            .field("bot", &self.bot)
            .finish()
    }
}

// impl<B> Debug for BotState<B> {s
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         f.debug_struct("BotState")
//             .field("client", &self.client)
//             .field("cache", &self.cache)
//             .finish()
//     }
// }

/// The most important trait to implement, as it determines how to connect your bot to Discord,
/// what Slash Commands to send to Discord, and how to handle the various events that a Discord bot
/// can witness.
///
///
/// ```rust
/// # use discorsd::{Bot, async_trait};
/// # use discorsd::shard::model::{Identify, UpdateStatus, Activity, ActivityType};
/// # use discorsd::shard::intents::Intents;
/// struct MyBot {
///     token: String
/// }
///
/// #[async_trait]
/// impl Bot for MyBot {
///     fn token(&self) -> String { self.token.clone() }
///
///     fn identify(&self) -> Identify {
///         Identify::new(self.token())
///             .presence(UpdateStatus::with_activity(
///                         // "listening to /help"
///                         Activity::for_bot("/help", ActivityType::Listening)
///             ))
///     }
/// }
/// ```
#[allow(unused)]
#[async_trait]
pub trait Bot: Send + Sync + Sized {
    /// Register your Discord bot's token. This is the only method you are required to implement,
    /// though your bot will be very boring if you don't implement any other methods.
    fn token(&self) -> String;

    /// How to identify this bot to discord. Defaults to do nothing but set the bot's token and
    /// accept all [`Intents`](crate::shard::intents::Intents).
    ///
    /// See [`Identify`](Identify) for more information.
    fn identify(&self) -> Identify { Identify::new(self.token()) }

    /// All of the bot's global commands as a static slice. This is called once when the bot
    /// receives the [`Ready`](crate::shard::dispatch::Ready) event, sending these commands to
    /// Discord and registering them in the bot's [`BotState`](crate::BotState) in order to run
    /// them when invoked.
    fn global_commands() -> &'static [&'static dyn SlashCommandRaw<Bot=Self>] { &[] }

    fn global_user_commands() -> &'static [&'static dyn UserCommand<Bot=Self>] { &[] }
    // todo add global_message_commands() and add shard\mod.rs global user and message commands

    fn guild_commands() -> Vec<Box<dyn SlashCommandRaw<Bot=Self>>> { Vec::new() }

    async fn ready(&self, state: Arc<BotState<Self>>) -> Result<(), BotError> { Ok(()) }

    async fn resumed(&self, state: Arc<BotState<Self>>) -> Result<(), BotError> { Ok(()) }

    async fn guild_create(&self, guild: Guild, state: Arc<BotState<Self>>) -> Result<(), BotError> { Ok(()) }

    async fn message_create(&self, message: Message, state: Arc<BotState<Self>>) -> Result<(), BotError> { Ok(()) }

    async fn message_update(&self, message: Message, state: Arc<BotState<Self>>, updates: MessageUpdate) -> Result<(), BotError> { Ok(()) }

    async fn interaction(&self, interaction: interaction::Interaction, state: Arc<BotState<Self>>) -> Result<(), BotError> {
        Self::handle_interaction(interaction, state).await
    }

    async fn reaction(&self, reaction: ReactionUpdate, state: Arc<BotState<Self>>) -> Result<(), BotError> { Ok(()) }

    async fn integration_update(&self, guild: GuildId, integration: Integration, state: Arc<BotState<Self>>) -> Result<(), BotError> { Ok(()) }

    async fn role_create(&self, guild: GuildId, role: Role, state: Arc<BotState<Self>>) -> Result<(), BotError> { Ok(()) }

    async fn role_update(&self, guild: GuildId, role: Role, state: Arc<BotState<Self>>) -> Result<(), BotError> { Ok(()) }

    async fn error(&self, error: BotError, state: Arc<BotState<Self>>) {
        error!("{}", error.display_error(&state).await);
    }
}

/// Extension methods for [Bot]s.
#[async_trait]
pub trait BotExt: Bot + 'static {
    /// Run the bot. Should likely be the last line of `main`.
    async fn run(self) -> shard::ShardResult<()> {
        BotRunner::from(self).run().await
    }

    /// Respond to an interaction with the matching [SlashCommand]. Should likely be used in the
    /// [Bot::interaction](Bot::interaction) method.
    async fn handle_interaction(interaction: interaction::Interaction, state: Arc<BotState<Self>>) -> Result<(), BotError> {
        match interaction {
            interaction::Interaction::Ping => println!("PING!"),
            interaction::Interaction::ApplicationCommand(data) => {
                let interaction::InteractionData {
                    id: interaction_id,
                    application_id,
                    token,
                    channel_id,
                    data,
                    message: _,
                    user,
                    app_permissions,
                    locale
                } = data;
                match data {
                    ApplicationCommandData::SlashCommand { id, name, options } => {
                        let interaction = InteractionUse::new(
                            interaction_id,
                            application_id,
                            AppCommandData { command: id, command_name: name },
                            channel_id,
                            user,
                            token,
                        );
                        let global_command = state.global_commands.get().unwrap().get(&id);
                        if let Some(command) = global_command {
                            command.run(Arc::clone(&state), interaction, options).await?;
                        } else {
                            let command = {
                                let guard = state.commands.read().await;
                                // todo fix this unwrap lol
                                let commands = guard.get(&interaction.guild().unwrap()).unwrap().read().await;
                                commands.get(&id).cloned()
                            };
                            if let Some(command) = command {
                                command.run(Arc::clone(&state), interaction, options).await?;
                            }
                        }
                    },
                    ApplicationCommandData::UserCommand { id, name, target_id, resolved } => {
                        let global_user_command = state.global_user_commands.get().unwrap().get(&id);
                        if let Some(command) = global_user_command {
                            let interaction = InteractionUse::new(
                                interaction_id,
                                application_id,
                                AppCommandData { command: id, command_name: name },
                                channel_id,
                                user,
                                token,
                            );
                            // todo double check this and rename variables?
                            let target_user = resolved.users.get(target_id);
                            if let Some(u) = target_user {
                                let guild_member = resolved.members.get(&target_id);
                                command.run(Arc::clone(&state), interaction, u.clone(), guild_member.cloned()).await?;
                            }
                        }
                    },
                    ApplicationCommandData::MessageCommand { id, name, target_id, resolved } => {
                        let global_message_command = state.global_message_commands.get().unwrap().get(&id);
                        if let Some(command) = global_message_command {
                            let interaction = InteractionUse::new(
                                interaction_id,
                                application_id,
                                AppCommandData { command: id, command_name: name },
                                channel_id,
                                user,
                                token,
                            );
                            // todo double check this and rename variables?
                            let target_message = resolved.messages.get(target_id);
                            if let Some(m) = target_message {
                                command.run(Arc::clone(&state), interaction, m.clone()).await?;
                            }
                        }
                    },
                }
            }
            interaction::Interaction::MessageComponent(data) => {
                let interaction::InteractionData {
                    id: interaction_id,
                    application_id,
                    token,
                    channel_id,
                    data,
                    message,
                    user,
                    app_permissions,
                    locale
                } = data;
                match data {
                    MessageComponentData::Button(data) => {
                        let command = state.buttons.read().unwrap().get(&data.custom_id).cloned();
                        if let Some(command) = command {
                            let interaction = InteractionUse::new(
                                interaction_id,
                                application_id,
                                data,
                                channel_id,
                                user,
                                token,
                            );
                            command.run(Arc::clone(&state), interaction).await?;
                        }
                    }
                    MessageComponentData::StringMenu(data)
                    | MessageComponentData::UserMenu(data)
                    | MessageComponentData::RoleMenu(data)
                    | MessageComponentData::MentionableMenu(data)
                    | MessageComponentData::ChannelMenu(data) => {
                        let command = state.menus.read().unwrap().get(&data.custom_id).cloned();
                        if let Some(command) = command {
                            let interaction = InteractionUse::new(
                                interaction_id,
                                application_id,
                                data,
                                channel_id,
                                user,
                                token,
                            );
                            command.run(Arc::clone(&state), interaction).await?;
                        }
                    }
                    MessageComponentData::TextInput => todo!(),
                }
            }
            interaction::Interaction::ApplicationCommandAutocomplete(_) => todo!(),
            interaction::Interaction::ModalSubmit(_) => todo!(),
        }
        Ok(())
    }
}

#[async_trait]
impl<B: Bot + 'static> BotExt for B {}

struct BotRunner<B: Bot + 'static> {
    shards: Vec<Shard<B>>,
}

impl<B: Bot + 'static> From<B> for BotRunner<B> {
    fn from(bot: B) -> Self {
        let state = Arc::new(BotState {
            client: DiscordClient::single(bot.token()),
            cache: Default::default(),
            bot,
            commands: Default::default(),
            command_names: Default::default(),
            global_commands: Default::default(),
            global_user_commands: Default::default(),
            global_message_commands: Default::default(),
            global_command_names: Default::default(),
            global_user_command_names: Default::default(),
            global_message_command_names: Default::default(),
            reaction_commands: Default::default(),
            buttons: Default::default(),
            menus: Default::default(),
            count: Default::default(),
        });
        // todo more than one shard
        let shard = Shard::new(Arc::clone(&state));
        Self {
            shards: vec![shard]
        }
    }
}

impl<B: Bot + 'static> BotRunner<B> {
    async fn run(self) -> shard::ShardResult<()> {
        let mut handles = Vec::new();
        for mut shard in self.shards {
            let handle = tokio::spawn(async move {
                (shard.shard_info, shard.run().await)
            });
            handles.push(handle);
        }
        // todo maybe this should be try_join or smth, so that if it can restart the second even if
        //  the first is still going?
        for handle in handles {
            match handle.await {
                Ok((id, _handle)) => {
                    error!("Shard {:?} finished (this should be unreachable?)", id);
                    // handle.unwrap();
                }
                Err(e) => {
                    error!("this is awkward, I didn't expect {}", e);
                }
            }
        }
        unreachable!()
        // Err(ShardError::Other("Shouldn't stop running".into()))
    }
}
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
use crate::commands::{ButtonCommand, MenuCommandRaw, MenuSelectData, ReactionCommand, SlashCommand, SlashCommandRaw};
use crate::errors::BotError;
use crate::http::DiscordClient;
use crate::model::commands::{ButtonPressData, InteractionUse, SlashCommandData};
use crate::model::components::{Button, ComponentId, SelectMenu};
use crate::model::guild::{Guild, Integration};
use crate::model::ids::*;
use crate::model::message::Message;
use crate::model::new_interaction;
use crate::model::new_interaction::{ApplicationCommandData, MenuData, MessageComponentData};
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
    /// The global [`SlashCommand`](SlashCommand) ids your bot has created, by name.
    pub global_command_names: OnceCell<HashMap<&'static str, CommandId>>,
    /// The [`ReactionCommand`](ReactionCommand)s your bot is using.
    pub reaction_commands: RwLock<Vec<Box<dyn ReactionCommand<B>>>>,
    pub buttons: std::sync::RwLock<HashMap<ComponentId, Box<dyn ButtonCommand<Bot=B>>>>,
    pub menus: std::sync::RwLock<HashMap<ComponentId, Box<dyn MenuCommandRaw<Bot=B>>>>,
    pub count: AtomicUsize,
}

impl<B: Send + Sync + 'static> BotState<B> {
    pub(crate) fn make_button(&self, button: Box<dyn ButtonCommand<Bot=B>>) -> Button {
        let count = self.count.fetch_add(1, Ordering::Relaxed);
        let id: ComponentId = count.to_string().into();
        let component = Button {
            style: button.style(),
            label: Some(button.label()),
            emoji: button.emoji(),
            custom_id: Some(id.clone()),
            url: None,
            disabled: false,
        };
        self.buttons.write().unwrap().insert(id, button);
        component
    }

    pub(crate) fn make_string_menu(&self, menu: Box<dyn MenuCommandRaw<Bot=B>>) -> SelectMenu<String> {
        let count = self.count.fetch_add(1, Ordering::Relaxed);
        let id: ComponentId = count.to_string().into();
        let (min_values, max_values) = menu.num_values();
        let component = SelectMenu {
            custom_id: id.clone(),
            options: menu.options(),
            channel_types: (),
            placeholder: menu.placeholder(),
            min_values,
            max_values,
            disabled: menu.disabled(),
        };
        self.menus.write().unwrap().insert(id, menu);
        component
    }

    // pub fn button<Btn: ButtonCommand<Bot=B>>(&self, id: &ComponentId) -> Option<&mut Btn> {
    //     self.buttons.write().unwrap()
    //         .get_mut(id)
    //         .and_then(|btn| btn.downcast_mut::<Btn>())
    // }
    //
    // pub fn menu<M: MenuCommand<Bot=B>>(&self, id: &ComponentId) -> Option<&mut M> {
    //     self.menus.write().unwrap()
    //         .get_mut(id)
    //         .and_then(|menu| menu.downcast_mut::<M>())
    // }
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
            global_command_names: Default::default(),
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

    fn guild_commands() -> Vec<Box<dyn SlashCommandRaw<Bot=Self>>> { Vec::new() }

    async fn ready(&self, state: Arc<BotState<Self>>) -> Result<(), BotError> { Ok(()) }

    async fn resumed(&self, state: Arc<BotState<Self>>) -> Result<(), BotError> { Ok(()) }

    async fn guild_create(&self, guild: Guild, state: Arc<BotState<Self>>) -> Result<(), BotError> { Ok(()) }

    async fn message_create(&self, message: Message, state: Arc<BotState<Self>>) -> Result<(), BotError> { Ok(()) }

    async fn message_update(&self, message: Message, state: Arc<BotState<Self>>, updates: MessageUpdate) -> Result<(), BotError> { Ok(()) }

    async fn interaction(&self, interaction: new_interaction::Interaction, state: Arc<BotState<Self>>) -> Result<(), BotError> {
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
    async fn handle_interaction(interaction: new_interaction::Interaction, state: Arc<BotState<Self>>) -> Result<(), BotError> {
        // println!("interaction = {:#?}", interaction);
        match interaction {
            new_interaction::Interaction::Ping => println!("PING!"),
            new_interaction::Interaction::ApplicationCommand(data) => {
                let new_interaction::InteractionData {
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
                            SlashCommandData { command: id, command_name: name },
                            channel_id,
                            user,
                            token,
                        );
                        println!("options = {:?}", options);
                        println!("interaction = {:?}", interaction);
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
                    }
                    ApplicationCommandData::UserCommand { .. } => todo!(),
                    ApplicationCommandData::MessageCommand { .. } => todo!(),
                }
            }
            new_interaction::Interaction::MessageComponent(data) => {
                let new_interaction::InteractionData {
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
                    MessageComponentData::Button { custom_id } => {
                        let command = state.buttons.read().unwrap().get(&custom_id).cloned();
                        if let Some(command) = command {
                            let interaction = InteractionUse::new(
                                interaction_id,
                                application_id,
                                ButtonPressData { custom_id },
                                channel_id,
                                user,
                                token,
                            );
                            command.run(Arc::clone(&state), interaction).await?;
                        }
                    }
                    MessageComponentData::StringMenu(MenuData { custom_id, values }) => {
                        let command = state.menus.read().unwrap().get(&custom_id).cloned();
                        if let Some(command) = command {
                            let interaction = InteractionUse::new(
                                interaction_id,
                                application_id,
                                MenuSelectData { custom_id, values },
                                channel_id,
                                user,
                                token,
                            );
                            command.run(Arc::clone(&state), interaction).await?;
                        }
                    }
                    MessageComponentData::TextInput => todo!(),
                    MessageComponentData::UserMenu(_) => todo!(),
                    MessageComponentData::RoleMenu(_) => todo!(),
                    MessageComponentData::MentionableMenu(_) => todo!(),
                    MessageComponentData::ChannelMenu(_) => todo!(),
                }
            }
            new_interaction::Interaction::ApplicationCommandAutocomplete(_) => todo!(),
            new_interaction::Interaction::ModalSubmit(_) => todo!(),
        }
        // let Interaction {
        //     id,
        //     application_id,
        //     kind: _kind,
        //     data,
        //     source,
        //     channel_id,
        //     token,
        // } = interaction;
        // match data {
        //     InteractionData::ApplicationCommand(data) => {
        //         let interaction = InteractionUse::new(
        //             id,
        //             application_id,
        //             SlashCommandData { command: data.id, command_name: data.name },
        //             channel_id,
        //             source,
        //             token,
        //         );
        //         let command = state.global_commands.get().unwrap().get(&data.id);
        //         if let Some(command) = command {
        //             command.run(Arc::clone(&state), interaction, data.options).await?;
        //         } else {
        //             let command = {
        //                 let guard = state.commands.read().await;
        //                 // todo fix this unwrap lol
        //                 let commands = guard.get(&interaction.guild().unwrap()).unwrap().read().await;
        //                 commands.get(&data.id).cloned()
        //             };
        //             if let Some(command) = command {
        //                 command.run(Arc::clone(&state), interaction, data.options).await?;
        //             }
        //         }
        //     }
        //     InteractionData::MessageComponentCommand(data) => {
        //         match data.component_type {
        //             2 => {
        //                 let command = state.buttons.read().unwrap().get(&data.custom_id).cloned();
        //                 if let Some(command) = command {
        //                     let interaction = InteractionUse::new(
        //                         id,
        //                         application_id,
        //                         ButtonPressData { custom_id: data.custom_id },
        //                         channel_id,
        //                         source,
        //                         token,
        //                     );
        //                     command.run(Arc::clone(&state), interaction).await?;
        //                 }
        //             }
        //             3 => {
        //                 let command = state.menus.read().unwrap().get(&data.custom_id).cloned();
        //                 if let Some(command) = command {
        //                     let interaction = InteractionUse::new(
        //                         id,
        //                         application_id,
        //                         MenuSelectData { custom_id: data.custom_id, values: data.values },
        //                         channel_id,
        //                         source,
        //                         token,
        //                     );
        //                     command.run(Arc::clone(&state), interaction).await?;
        //                 }
        //             }
        //             _bad => todo!(),
        //         }
        //     }
        //     InteractionData::MessageCommand { .. } => todo!(),
        //     InteractionData::UserCommand { .. } => todo!(),
        // }

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
            global_command_names: Default::default(),
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
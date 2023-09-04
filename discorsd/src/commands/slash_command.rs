use std::borrow::Cow;
use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use downcast_rs::{Downcast, impl_downcast};
use dyn_clone::DynClone;

use crate::{Bot, BotState};
use crate::errors::{BotError, CommandParseErrorInfo};
use crate::http::ClientResult;
use crate::model::command::{ApplicationCommand, Command, CommandOption};
use crate::model::guild::GuildId;
use crate::model::ids::CommandId;
use crate::model::interaction::InteractionOption;
use crate::model::interaction_response::ephemeral;

pub use crate::model::commands::*;

/// The trait to implement to define a Slash Command.
///
/// All structs that implement this must implement `Clone` and `Debug`, so you'll likely want to
/// `#[derive(Clone, Debug)]` on your command.
///
/// The [`run`](SlashCommand::run) method of this trait is `async`, so all implementations of this trait must be
/// annotated with `#[async_trait]`, which is re-exported by `discorsd`.
///
/// For example, a command that adds a yourself (or another user) to a game:
/// ```rust
/// use discorsd::commands::*;
/// # use std::borrow::Cow;
/// # use discorsd::BotState;
/// # use std::sync::Arc;
/// # use discorsd::errors::{BotError, CommandParseError};
/// # use discorsd::model::commands::*;
/// # use discorsd::model::ids::UserId;
/// # struct MyBot;
///
/// #[derive(Clone, Copy, Debug)]
/// struct AddMeCommand;
///
/// #[derive(Debug, command_data_derive::CommandDataChoices)]
/// enum Game { #[command(default)] TicTacToe, Hangman, Pong, }
///
/// #[derive(Debug, command_data_derive::CommandData)]
/// struct AddMeData {
///     #[command(default, desc = "The game to add you to, or TicTacToe if not specified")]
///     game: Game,
///     #[command(desc = "Add someone else to the game")]
///     player: Option<UserId>,
/// }
///
/// #[discorsd::async_trait]
/// impl SlashCommand for AddMeCommand {
///     type Bot = MyBot;
///     type Data = AddMeData;
///     type Use = Used;
///     const NAME: &'static str = "addme";
///
///     fn description(&self) -> Cow<'static, str> {
///         "Add yourself (or someone else) to a game".into()
///     }
///
///     async fn run(&self,
///                  state: Arc<BotState<MyBot>>,
///                  interaction: InteractionUse<AppCommandData, Unused>,
///                  data: Self::Data
///     ) -> Result<InteractionUse<AppCommandData, Self::Use>, BotError<Self::Bot::Error>> {
///         interaction.respond(state, format!("received data: {:?}", data))
///                    .await
///                    .map_err(|e| e.into())
///     }
/// }
/// ```
#[async_trait]
pub trait SlashCommand: Sized + Send + Sync + Debug + Downcast + DynClone + SlashCommandRaw<Bot=<Self as SlashCommand>::Bot> {
    /// Your discord bot. Should probably implement [`Bot`](crate::Bot).
    type Bot: Bot;
    /// The type of data this command has. Can be `()` for commands which have no arguments.
    /// Otherwise, the best way to implement `CommandData` for your data is with
    /// `#[derive(CommandData)]`.
    type Data: CommandData<Self> + Send;
    /// What the state of the interaction should be after performing the [`run`](Self::run) method.
    ///
    /// Most of the time, this will be [`Used`](Used), meaning the [`run`](Self::run) method
    /// responded to and/or deleted the interaction. Alternatively, this can be
    /// [`Deferred`](Deferred) if the [`run`](Self::run) method only
    /// [`defer`](InteractionUse::<Data, Unused>::defer)s the interaction, to automatically delete the
    /// interaction after the [`run`](Self::run) method finishes.
    type Use: NotUnused + Send;

    /// The name that this command is invoked in Discord with.
    const NAME: &'static str;

    /// The description of this command that is displayed in the Command picker in Discord.
    ///
    /// `Cow<'static, str>` implements both `From<'static str>` and `From<String>`, you will
    /// probably want to use one of these to turn a string `into` a `Cow`.
    fn description(&self) -> Cow<'static, str>;

    /// All members of a guild this command is in are able to use it. Defaults to `true`.
    fn default_permissions(&self) -> bool { true }

    // todo should this be a method??? or just invoked in the impl of SCR?
    /// The structure of the command sent to Discord. By default, uses [`Data`](Self::Data)'s impl
    /// of [`CommandData::make_args`](CommandData::make_args), but can be overridden. Note: if you
    /// override this method, you *MUST* ensure that the command structure is compatible with/can be
    /// deserialized into [`Data`](Self::Data).
    fn options(&self) -> Vec<CommandOption> {
        <Self::Data as CommandData<Self>>::VecArg::wrap(Self::Data::make_args(self))
    }

    /// This method is called every time this command is invoked, and must suitably use the
    /// interaction.
    async fn run(&self,
                 state: Arc<BotState<<Self as SlashCommand>::Bot>>,
                 interaction: InteractionUse<AppCommandData, Unused>,
                 data: Self::Data,
    ) -> Result<InteractionUse<AppCommandData, Self::Use>, BotError<<<Self as SlashCommand>::Bot as Bot>::Error>>;
}

#[async_trait]
impl<SC: SlashCommand> SlashCommandRaw for SC
    where InteractionUse<AppCommandData, <Self as SlashCommand>::Use>: FinalizeInteraction<AppCommandData>,
          <Self as SlashCommand>::Bot: Send + Sync,
{
    type Bot = <Self as SlashCommand>::Bot;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn command(&self) -> Command {
        Command::slash_command(
            Self::NAME,
            self.description(),
            self.options(),
            // self.default_permissions(),
        )
    }

    async fn run(&self,
                 state: Arc<BotState<Self::Bot>>,
                 interaction: InteractionUse<AppCommandData, Unused>,
                 data: InteractionOption,
    ) -> Result<InteractionUse<AppCommandData, Used>, BotError<<Self::Bot as Bot>::Error>> {
        match <<Self as SlashCommand>::Data as CommandData<Self>>::Options::from_data_option(data) {
            Ok(options) => match <Self as SlashCommand>::Data::from_options(options) {
                Ok(data) => {
                    let self_use = SlashCommand::run(self, Arc::clone(&state), interaction, data).await?;
                    self_use.finalize(&state).await.map_err(Into::into)
                }
                Err(error) => {
                    let interaction = interaction.respond(
                        state,
                        ephemeral(format!("Error parsing command: ```rs\n{error:?}```")),
                    ).await?;
                    Err(CommandParseErrorInfo {
                        name: interaction.data.command_name,
                        id: interaction.data.command,
                        source: interaction.source,
                        error,
                    }.into())
                }
            },
            Err(error) => {
                let interaction = interaction.respond(
                    state,
                    ephemeral(format!("Error parsing command: ```rs\n{error:?}```")),
                ).await?;
                Err(CommandParseErrorInfo {
                    name: interaction.data.command_name,
                    id: interaction.data.command,
                    source: interaction.source,
                    error,
                }.into())
            }
        }
    }
}

/// The lower level Slash Command trait. You should always prefer to implement [SlashCommand]
/// instead of this.
///
/// [SlashCommand] is much more easily customizable while also being simpler to implement (you don't
/// have to manually create the [Command] sent to Discord, nor do you have to manually parse the
/// interaction received when the command is invoked).
///
/// This is what is stored in [BotState](crate::bot::BotState), which means that it can't have
/// varying associated types ([Data](SlashCommand::Data) and [Use](SlashCommand::Use)) since it has
/// to be object safe.
///
/// This is implemented for all types which implement [SlashCommand].
#[async_trait]
pub trait SlashCommandRaw: Send + Sync + Debug + Downcast + DynClone {
    type Bot: Bot;

    fn name(&self) -> &'static str;

    fn command(&self) -> Command;

    async fn run(&self,
                 state: Arc<BotState<Self::Bot>>,
                 interaction: InteractionUse<AppCommandData, Unused>,
                 data: InteractionOption,
    ) -> Result<InteractionUse<AppCommandData, Used>, BotError<<Self::Bot as Bot>::Error>>;
}

impl_downcast!(SlashCommandRaw assoc Bot);
impl<'clone, B> Clone for Box<dyn SlashCommandRaw<Bot=B> + 'clone> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}

/// Extension trait for [SlashCommand]s to edit them
#[async_trait]
pub trait SlashCommandExt: SlashCommandRaw {
    /// Edit `command` by id, updating its description, options, and default_permissions.
    ///
    /// Note: the command's name is not edited.
    async fn edit_command<State, B>(
        &mut self,
        state: State,
        guild: GuildId,
        command: CommandId,
    ) -> ClientResult<ApplicationCommand>
        where B: 'static + Send + Sync,
              State: AsRef<BotState<B>> + Send,
    {
        match self.command() {
            Command::SlashCommand { name, name_localizations, description, description_localizations, options } => {
                let state = state.as_ref();
                state.client.edit_guild_command(
                    state.application_id(),
                    guild,
                    command,
                    None,
                    Some(description.as_ref()),
                    Some(options),
                    // todo
                    None,
                    // Some(default_permission),
                ).await
            }
            Command::UserCommand { .. } => unreachable!(),
            Command::MessageCommand { .. } => unreachable!(),
        }
    }
}

#[async_trait]
impl<C: SlashCommandRaw> SlashCommandExt for C {}

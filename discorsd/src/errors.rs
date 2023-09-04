//! Error handling for `discorsd`, mainly though the [`BotError`](BotError) enum.

use std::error::Error;
use std::fmt::{self, Debug, Display};
use std::ops::Range;

use thiserror::Error;

use crate::{Bot, BotState, serde_utils};
use crate::commands::slash_command::SlashCommandRaw;
use crate::http::{ClientError, DisplayClientError};
use crate::model::DiscordError;
use crate::model::ids::*;
use crate::model::interaction::{DmUser, GuildUser, InteractionDataOption, InteractionUser};

#[derive(Error, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum BotError<E> {
    #[error(transparent)]
    Custom(E),
    #[error(transparent)]
    Client(#[from] ClientError),
    #[error(transparent)]
    CommandParse(#[from] CommandParseErrorInfo),
    #[error("Error converting `chrono::time::Duration` to `std::time::Duration`")]
    Chrono,
}

// since GameError is an enum, want to be able to Into its variants into BotError (maybe others too)
// todo better name
#[macro_export]
macro_rules! bot_error_from {
    ($e2:ty => E = $e1:ty) => {
        impl From<$e2> for $crate::errors::BotError<$e1> {
            fn from(e2: $e2) -> Self {
                Self::from(<$e1>::from(e2))
            }
        }
    };
    ($e2:ty => $e1:ty) => {
        impl<E: ::std::error::Error> From<$e2> for $crate::errors::BotError<E> {
            fn from(e2: $e2) -> Self {
                Self::from(<$e1>::from(e2))
            }
        }
    };
}
bot_error_from!(reqwest::Error => ClientError);
bot_error_from!(serde_utils::Error => ClientError);
bot_error_from!(std::io::Error => ClientError);
bot_error_from!(DiscordError => ClientError);

impl<E: Error + Sync> BotError<E> {
    pub async fn display_error<B: Bot + Send + Sync>(&self, state: &BotState<B>) -> DisplayBotError<'_, E> {
        match self {
            Self::Custom(e) => DisplayBotError::Custom(e),
            Self::Client(e) => DisplayBotError::Client(e.display_error(state).await),
            Self::CommandParse(e) => DisplayBotError::CommandParse(e.display_error(state).await),
            Self::Chrono => DisplayBotError::Chrono,
        }
    }
}

pub enum DisplayBotError<'a, E> {
    Custom(&'a E),
    Client(DisplayClientError<'a>),
    CommandParse(String),
    Chrono,
}

impl<E: Error> Display for DisplayBotError<'_, E> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Client(e) => write!(f, "{e}"),
            Self::Custom(e) => write!(f, "{e}"),
            Self::CommandParse(e) => f.write_str(e),
            Self::Chrono => f.write_str("Error converting `chrono::time::Duration` to `std::time::Duration`"),
        }
    }
}

#[derive(Error, Debug)]
pub struct CommandParseErrorInfo {
    pub name: String,
    pub id: CommandId,
    pub source: InteractionUser,
    pub error: CommandParseError,
}

impl CommandParseErrorInfo {
    #[allow(clippy::missing_panics_doc)]
    pub async fn display_error<B: Bot + Send + Sync>(&self, state: &BotState<B>) -> String {
        let source = match &self.source {
            InteractionUser::Guild(GuildUser { id, member, locale }) => if let Some(guild) = state.cache.guild(id).await {
                format!(
                    "guild `{}` ({}), used by `{}` ({})",
                    guild.name.as_deref().unwrap_or("null"), guild.id, member.nick_or_name(), member.id()
                )
            } else {
                format!(
                    "unknown guild `{}`, used by `{}` ({})",
                    id, member.nick_or_name(), member.id()
                )
            },
            InteractionUser::Dm(DmUser { user }) => format!(
                "dm with `{}#{}` ({})",
                user.username, user.discriminator, user.id
            ),
        };
        match &self.source {
            InteractionUser::Guild(GuildUser { id, .. }) => {
                let guard = state.slash_commands.read().await;
                if let Some(guild_lock) = guard.get(id) {
                    let guard = guild_lock.read().await;
                    self.command_fail_message(&source, guard.get(&self.id).map(|c| &**c))
                } else {
                    format!(
                        "Failed to parse command `{}` in {}, which has no commands: {:?}",
                        self.id, source, self.error,
                    )
                }
            }
            InteractionUser::Dm(_) => {
                let global = state.global_slash_commands.get().unwrap();
                self.command_fail_message(&source, global.get(&self.id).copied())
            }
        }
    }

    fn command_fail_message<B: Bot + 'static>(&self, source: &str, command: Option<&dyn SlashCommandRaw<Bot=B>>) -> String {
        if let Some(command) = command {
            format!(
                "Failed to parse command `{}` ({}) in {}: {:?}",
                command.name(), self.id, source, self.error
            )
        } else {
            format!(
                "Failed to parse unknown command `{}` in {}: {:?}",
                self.id, source, self.error,
            )
        }
    }
}

impl Display for CommandParseErrorInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

#[derive(Debug)]
pub enum CommandParseError {
    BadType(OptionTypeError),
    FromInt(i64),
    UnknownOption(UnknownOption),
    EmptyOption(String),
    BadOrder(String, usize, Range<usize>),
    /// found a vararg option when it was not expected
    UnexpectedVararg(String, usize),
    /// found a single option when expecting to find a vararg
    UnexpectedSingleOption(String, usize),
    MissingOption(String),
    /// Command named `String` didn't have a subcommand option
    NoSubtype(String),
    /// InteractionDataOption::Group(_) when parsing data for an struct
    BadGroupOccurrence,
    /// InteractionDataOption::Command(_) when parsing data for an struct
    BadCommandOccurrence,
    /// InteractionDataOption::Values(_) when parsing data for an enum
    BadValueOccurrence,
}

#[derive(Debug)]
pub struct OptionTypeError {
    pub value: InteractionDataOption,
    pub desired: CommandOptionTypeParsed,
}

/// like [`ApplicationCommandOptionType`](crate::commands::ApplicationCommandOptionType), but more
/// specifically for single option types and with more options (that have been further parsed, such
/// as unsigned ints or message id).
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum CommandOptionTypeParsed {
    String,
    I64,
    F64,
    Usize,
    Boolean,
    UserId,
    ChannelId,
    RoleId,
    MentionableId,
    // todo Attachment?
}

impl From<OptionTypeError> for CommandParseError {
    fn from(ot: OptionTypeError) -> Self {
        Self::BadType(ot)
    }
}

#[derive(Debug)]
pub struct UnknownOption {
    pub name: String,
    pub options: &'static [&'static str],
}
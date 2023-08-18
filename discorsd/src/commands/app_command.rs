use std::fmt::Debug;
use std::sync::Arc;

use async_trait::async_trait;
use downcast_rs::{Downcast, impl_downcast};
use dyn_clone::DynClone;

use crate::bot::BotState;
use crate::errors::BotError;
use crate::model::command::Command;
pub use crate::model::commands::*;
use crate::model::interaction::PartialGuildMember;
use crate::model::message::Message;
use crate::model::user::User;

#[async_trait]
pub trait UserCommand: Send + Sync + Debug + DynClone + Downcast {
    type Bot: Send + Sync;

    // todo add user command name field? (const prevents downcast)
    // const NAME: &'static str;

    // todo update name()?
    fn name(&self) -> &'static str;

    fn command(&self) -> Command {
        Command::user_command(self.name())
    }

    async fn run(&self,
                 state: Arc<BotState<Self::Bot>>,
                 interaction: InteractionUse<AppCommandData, Unused>,
                 target: User,
                 guild_member: Option<PartialGuildMember>,
    ) -> Result<InteractionUse<AppCommandData, Used>, BotError>;
}

impl_downcast!(UserCommand assoc Bot);
impl<'clone, B> Clone for Box<dyn UserCommand<Bot=B> + 'clone> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}

#[async_trait]
pub trait MessageCommand: Send + Sync + Debug + DynClone + Downcast {
    type Bot: Send + Sync;

    // todo add message command name field? (const prevents downcast)
    // const NAME: &'static str;

    // todo update name()?
    fn name(&self) -> &'static str;

    fn command(&self) -> Command {
        Command::message_command(self.name())
    }

    async fn run(&self,
                 state: Arc<BotState<Self::Bot>>,
                 interaction: InteractionUse<AppCommandData, Unused>,
                 target: Message,
    ) -> Result<InteractionUse<AppCommandData, Used>, BotError>;
}

impl_downcast!(MessageCommand assoc Bot);
impl<'clone, B> Clone for Box<dyn MessageCommand<Bot=B> + 'clone> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}

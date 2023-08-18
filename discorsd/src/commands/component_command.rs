use async_trait::async_trait;
use dyn_clone::DynClone;
use downcast_rs::{Downcast, impl_downcast};
use std::sync::Arc;
use std::str::FromStr;
use std::fmt::Debug;
use crate::BotState;
use crate::errors::BotError;
use crate::model::interaction::{ButtonPressData, MenuSelectData, MenuSelectDataRaw};
pub use crate::model::commands::*;

/// Not url buttons
#[async_trait]
pub trait ButtonCommand: Send + Sync + DynClone + Downcast {
    type Bot: Send + Sync;

    async fn run(&self,
                 state: Arc<BotState<Self::Bot>>,
                 interaction: InteractionUse<ButtonPressData, Unused>,
    ) -> Result<InteractionUse<ButtonPressData, Used>, BotError>;
}

impl_downcast!(ButtonCommand assoc Bot);
impl<'clone, B> Clone for Box<dyn ButtonCommand<Bot=B> + 'clone> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}

#[async_trait]
pub trait MenuCommandRaw: Send + Sync + DynClone + Downcast {
    type Bot: Send + Sync;

    async fn run(&self,
                 state: Arc<BotState<Self::Bot>>,
                 interaction: InteractionUse<MenuSelectDataRaw, Unused>,
    ) -> Result<InteractionUse<MenuSelectData, Used>, BotError>;
}

impl_downcast!(MenuCommandRaw assoc Bot);
impl<'clone, B> Clone for Box<dyn MenuCommandRaw<Bot=B> + 'clone> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}

#[async_trait]
pub trait MenuCommand: Send + Sync + DynClone + Downcast {
    type Bot: Send + Sync;

    type Data: MenuData + Send;

    async fn run(&self,
                 state: Arc<BotState<Self::Bot>>,
                 interaction: InteractionUse<MenuSelectData, Unused>,
                 data: Vec<Self::Data>,
    ) -> Result<InteractionUse<MenuSelectData, Used>, BotError>;
}

#[async_trait]
impl<M: MenuCommand> MenuCommandRaw for M
    where <M::Data as FromStr>::Err: Debug
{
    type Bot = M::Bot;

    async fn run(&self,
                 state: Arc<BotState<Self::Bot>>,
                 InteractionUse { id, application_id, data, channel, source, token, _priv }: InteractionUse<MenuSelectDataRaw, Unused>,
    ) -> Result<InteractionUse<MenuSelectData, Used>, BotError> {
        let interaction = InteractionUse {
            id,
            application_id,
            data: MenuSelectData {
                custom_id: data.custom_id,
                resolved: data.resolved,
            },
            channel,
            source,
            token,
            _priv,
        };
        let data = data.values.into_iter()
            .map(|string| string.parse())
            // todo handle errors better maybe
            .map(Result::unwrap)
            .collect();
        M::run(self, state, interaction, data).await
    }
}

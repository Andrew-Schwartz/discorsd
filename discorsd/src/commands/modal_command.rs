use std::borrow::Cow;
use std::convert::Infallible;
use std::num::ParseIntError;
use std::sync::Arc;

use async_trait::async_trait;
use downcast_rs::{Downcast, impl_downcast};
use dyn_clone::DynClone;
use itertools::Itertools;

use crate::{Bot, BotState};
use crate::errors::BotError;
pub use crate::model::commands::*;
use crate::model::components::ComponentId;
use crate::model::interaction::{ActionRowData, MessageComponentData, ModalSubmitData, TextSubmitData};

pub trait ArrayLen<const N: usize> {}

impl<const N: usize> ArrayLen<N> for Vec<String> {}

impl<const N: usize> ArrayLen<N> for [String; N] {}

impl ArrayLen<1> for String {}

// todo parse char error, bool, etc?
pub trait DisplayFromStrErr {
    // (from_str)_err not from_(str_err)
    #[allow(clippy::wrong_self_convention)]
    fn from_str_err(self, s: &str, field: &'static str) -> Cow<'static, str>;
}

impl DisplayFromStrErr for Infallible {
    fn from_str_err(self, _s: &str, _field: &'static str) -> Cow<'static, str> {
        match self {  }
    }
}

impl DisplayFromStrErr for ParseIntError {
    fn from_str_err(self, s: &str, field: &'static str) -> Cow<'static, str> {
        format!("error parsing `{field}` = `{s}` as number: `{self}`").into()
    }
}

pub trait ModalValues: Sized {
    fn from_vec(vec: Vec<String>) -> Result<Self, Cow<'static, str>>;
}

impl ModalValues for Vec<String> {
    fn from_vec(vec: Vec<String>) -> Result<Self, Cow<'static, str>> {
        Ok(vec)
    }
}

impl<const N: usize> ModalValues for [String; N] {
    fn from_vec(vec: Vec<String>) -> Result<Self, Cow<'static, str>> {
        Ok(vec.try_into()
            .expect("always has the right number of fields"))
    }
}

#[allow(clippy::use_self)]
impl ModalValues for String {
    fn from_vec(mut vec: Vec<String>) -> Result<Self, Cow<'static, str>> {
        match vec.len() {
            1 => Ok(vec.remove(0)),
            _ => unreachable!(),
        }
    }
}

#[async_trait]
pub trait ModalCommand: Send + Sync + DynClone + Downcast + ModalCommandRaw<Bot=<Self as ModalCommand>::Bot> {
    type Bot: Bot + Send + Sync;
    type Values: ModalValues;

    async fn run(&self,
                 state: Arc<BotState<<Self as ModalCommand>::Bot>>,
                 interaction: InteractionUse<ComponentId, Unused>,
                 values: Self::Values,
    ) -> Result<InteractionUse<ComponentId, Used>, BotError<<<Self as ModalCommand>::Bot as Bot>::Error>>;
}

#[async_trait]
impl<MC: ModalCommand> ModalCommandRaw for MC
    where <Self as ModalCommand>::Values: Send,
{
    type Bot = <Self as ModalCommand>::Bot;

    async fn run(
        &self,
        state: Arc<BotState<Self::Bot>>,
        interaction: InteractionUse<ModalSubmitData, Unused>,
    ) -> Result<InteractionUse<ComponentId, Used>, BotError<<Self::Bot as Bot>::Error>> {
        let InteractionUse { id, application_id, data, channel, source, token, _priv } = interaction;
        let values = data.components.into_iter()
            .map(|ActionRowData::ActionRow { mut components }| {
                assert_eq!(components.len(), 1);
                match components.remove(0) {
                    MessageComponentData::TextInput(TextSubmitData { custom_id, value }) => value,
                    _ => unreachable!("modals can only have text fields")
                }
            }).collect_vec();
        let interaction = InteractionUse {
            id,
            application_id,
            data: data.custom_id,
            channel,
            source,
            token,
            _priv: Default::default(),
        };
        match <Self as ModalCommand>::Values::from_vec(values) {
            Ok(values) => ModalCommand::run(
                self,
                state,
                interaction,
                values,
            ).await,
            Err(e) => interaction.respond(&state, format!("{e}"))
                .await
                .map_err(Into::into),
        }
    }
}

#[async_trait]
pub trait ModalCommandRaw: Send + Sync + DynClone + Downcast {
    type Bot: Bot + Send + Sync;

    async fn run(&self,
                 state: Arc<BotState<Self::Bot>>,
                 interaction: InteractionUse<ModalSubmitData, Unused>,
    ) -> Result<InteractionUse<ComponentId, Used>, BotError<<Self::Bot as Bot>::Error>>;
}

impl_downcast!(ModalCommandRaw assoc Bot);
impl<'clone, B> Clone for Box<dyn ModalCommandRaw<Bot=B> + 'clone> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}

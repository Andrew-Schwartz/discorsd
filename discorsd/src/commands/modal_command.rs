use std::fmt::{Debug, Display, Formatter};
use std::num::ParseIntError;
use std::sync::Arc;

use async_trait::async_trait;
use downcast_rs::{Downcast, impl_downcast};
use dyn_clone::DynClone;
use itertools::Itertools;

use crate::BotState;
use crate::errors::BotError;
pub use crate::model::commands::*;
use crate::model::components::ComponentId;
use crate::model::interaction::{ActionRowData, MessageComponentData, ModalSubmitData, TextSubmitData};

#[macro_export]
macro_rules! modal_values {
    ($data:ident => $n:literal; $vec:ident; $body:block) => {
        impl $crate::commands::ArrayLen<$n> for $data {}

        impl $crate::commands::ModalValues for $data {
            type Error = Vec<String>;

            fn from_vec(mut $vec: Vec<String>) -> Result<Self, Self::Error> {
                if $vec.len() != $n { return Err($vec) }
                else $body
            }
        }
    };
}

pub trait ArrayLen<const N: usize> {}

impl<const N: usize> ArrayLen<N> for Vec<String> {}

impl<const N: usize> ArrayLen<N> for [String; N] {}

impl ArrayLen<1> for String {}

#[derive(Debug, Eq, PartialEq, Clone)]
pub enum ModalParseError {
    MissingField(&'static str),
    ParseIntError(ParseIntError, String, &'static str),
}

impl Display for ModalParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ModalParseError::MissingField(field) => write!(f, "missing required field `{field}`"),
            ModalParseError::ParseIntError(e, val, field) => write!(f, "error parsing `{field}` = `{val}` as number: `{e}`"),
        }
    }
}

pub trait ModalValues: Sized {
    fn from_vec(vec: Vec<String>) -> Result<Self, ModalParseError>;
}

impl ModalValues for Vec<String> {
    fn from_vec(vec: Vec<String>) -> Result<Self, ModalParseError> {
        Ok(vec)
    }
}

impl<const N: usize> ModalValues for [String; N] {
    fn from_vec(vec: Vec<String>) -> Result<Self, ModalParseError> {
        vec.try_into()
            .map_err(|e| todo!())
    }
}

#[allow(clippy::use_self)]
impl ModalValues for String {
    fn from_vec(mut vec: Vec<String>) -> Result<Self, ModalParseError> {
        match vec.len() {
            1 => Ok(vec.remove(0)),
            _ => Err(todo!()),
        }
    }
}

#[async_trait]
pub trait ModalCommand: Send + Sync + DynClone + Downcast + ModalCommandRaw<Bot=<Self as ModalCommand>::Bot> {
    type Bot: Send + Sync;
    type Values: ModalValues;

    async fn run(&self,
                 state: Arc<BotState<<Self as ModalCommand>::Bot>>,
                 interaction: InteractionUse<ComponentId, Unused>,
                 values: Self::Values,
    ) -> Result<InteractionUse<ComponentId, Used>, BotError>;
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
    ) -> Result<InteractionUse<ComponentId, Used>, BotError> {
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
    type Bot: Send + Sync;

    async fn run(&self,
                 state: Arc<BotState<Self::Bot>>,
                 interaction: InteractionUse<ModalSubmitData, Unused>,
    ) -> Result<InteractionUse<ComponentId, Used>, BotError>;
}

impl_downcast!(ModalCommandRaw assoc Bot);
impl<'clone, B> Clone for Box<dyn ModalCommandRaw<Bot=B> + 'clone> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}

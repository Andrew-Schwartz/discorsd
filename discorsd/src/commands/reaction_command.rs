use async_trait::async_trait;
use std::fmt::Debug;
use downcast_rs::{Downcast, impl_downcast};
use dyn_clone::DynClone;
use std::sync::Arc;
use crate::{Bot, BotState};
use crate::errors::BotError;
use crate::shard::dispatch::ReactionUpdate;

/// Allow your bot to respond to reactions.
#[async_trait]
pub trait ReactionCommand: Send + Sync + Debug + Downcast + DynClone {
    type Bot: Bot;

    fn applies(&self, reaction: &ReactionUpdate) -> bool;

    async fn run(&self,
                 state: Arc<BotState<Self::Bot>>,
                 reaction: ReactionUpdate,
    ) -> Result<(), BotError<<Self::Bot as Bot>::Error>>;
}

impl_downcast!(ReactionCommand assoc Bot);
impl<'clone, B> Clone for Box<dyn ReactionCommand<Bot=B> + 'clone> {
    fn clone(&self) -> Self {
        dyn_clone::clone_box(&**self)
    }
}
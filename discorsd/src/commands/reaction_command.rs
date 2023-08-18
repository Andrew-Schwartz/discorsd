use async_trait::async_trait;
use std::fmt::Debug;
use downcast_rs::{Downcast, impl_downcast};
use dyn_clone::{clone_trait_object, DynClone};
use std::sync::Arc;
use crate::BotState;
use crate::errors::BotError;
use crate::shard::dispatch::ReactionUpdate;

/// Allow your bot to respond to reactions.
#[async_trait]
pub trait ReactionCommand<B>: Send + Sync + Debug + Downcast + DynClone {
    fn applies(&self, reaction: &ReactionUpdate) -> bool;

    async fn run(&self,
                 state: Arc<BotState<B>>,
                 reaction: ReactionUpdate,
    ) -> Result<(), BotError>;
}

impl_downcast!(ReactionCommand<B>);
// impl_downcast!(ReactionCommand<B> where B: Send + Sync);
clone_trait_object!(<B> ReactionCommand<B>);
// clone_trait_object!(<B> ReactionCommand<B> where B: Send + Sync);
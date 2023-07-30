use std::borrow::Cow;
use std::collections::{BTreeSet, HashSet};
use std::convert::Infallible;
use std::fmt::Debug;
use std::hash::{BuildHasher, Hash};
use std::marker::PhantomData;
use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::time::{Duration, Instant};

use crate::{BotState, utils};
use crate::cache::Cache;
use crate::commands::SlashCommandRaw;
use crate::errors::*;
use crate::http::{ClientResult, DiscordClient};
use crate::http::interaction::WebhookMessage;
use crate::model::{command, ids::*, interaction};
use crate::model::command::{Choice, CommandDataOption, CommandOption, OptionData, OptionType, SubCommandGroupOption, SubCommandOption};
use crate::model::components::{ComponentId, SelectMenuType, SelectOption};
use crate::model::guild::GuildMember;
use crate::model::interaction::{ButtonPressData, DataOption, DmUser, GuildUser, HasValue, InteractionDataOption, InteractionOption, InteractionUser, MenuSelectData, MenuSelectDataRaw, SubCommand, SubCommandGroup, Token};
use crate::model::interaction_response::{InteractionMessage, InteractionResponse};
use crate::model::message::{Attachment, Message};
use crate::model::user::User;

pub trait Usability: PartialEq {}

pub trait NotUnused: Usability {}

#[allow(clippy::empty_enum)]
#[derive(Debug, PartialEq)]
pub enum Unused {}

impl Usability for Unused {}

#[allow(clippy::empty_enum)]
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Deferred {}

impl Usability for Deferred {}

impl NotUnused for Deferred {}

#[allow(clippy::empty_enum)]
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum Used {}

impl Usability for Used {}

impl NotUnused for Used {}

pub trait InteractionPayload {}

// todo rename AppCommandData, ApplicationCommandData, ApplicationCommand
#[derive(Debug, Clone, PartialEq)]
pub struct AppCommandData {
    /// the id of the command being invoked
    pub command: CommandId,
    /// the name of the command being invoked
    pub command_name: String,
}

impl InteractionPayload for AppCommandData {}

pub trait ComponentData {}

impl<C: ComponentData> InteractionPayload for C {}

impl ComponentData for ButtonPressData {}

impl ComponentData for MenuSelectDataRaw {}

impl ComponentData for MenuSelectData {}

impl ComponentData for ComponentId {}

#[async_trait]
pub trait FinalizeInteraction<Data: InteractionPayload> {
    async fn finalize<B: Send + Sync + 'static>(self, state: &Arc<BotState<B>>) -> ClientResult<InteractionUse<Data, Used>>;
}

#[allow(clippy::use_self)]
#[async_trait]
impl<Data: InteractionPayload + Send> FinalizeInteraction<Data> for InteractionUse<Data, Used> {
    async fn finalize<B: Send + Sync + 'static>(self, _: &Arc<BotState<B>>) -> ClientResult<InteractionUse<Data, Used>> {
        Ok(self)
    }
}

#[allow(clippy::use_self)]
#[async_trait]
impl<Data: InteractionPayload + Send> FinalizeInteraction<Data> for InteractionUse<Data, Deferred> {
    async fn finalize<B: Send + Sync + 'static>(self, state: &Arc<BotState<B>>) -> ClientResult<InteractionUse<Data, Used>> {
        self.delete(state).await
    }
}

#[derive(Debug, Clone)]
pub struct InteractionUse<Data: InteractionPayload, Usability: self::Usability> {
    /// id of the interaction
    pub id: InteractionId,
    /// id of the application this interaction is for
    pub application_id: ApplicationId,
    // todo doc
    pub data: Data,
    /// the channel it was sent from
    pub channel: ChannelId,
    pub source: InteractionUser,
    /// a continuation token for responding to the interaction
    pub token: Token,
    pub(crate) _priv: PhantomData<Usability>,
}

impl<Data: InteractionPayload, Use: Usability> PartialEq for InteractionUse<Data, Use> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<Data: InteractionPayload, Use: Usability> Id for InteractionUse<Data, Use> {
    type Id = InteractionId;

    fn id(&self) -> Self::Id {
        self.id
    }
}

impl<D: InteractionPayload, U: Usability> InteractionUse<D, U> {
    pub fn guild(&self) -> Option<GuildId> {
        match &self.source {
            InteractionUser::Guild(gs) => Some(gs.id),
            InteractionUser::Dm(_) => None
        }
    }

    pub fn user(&self) -> &User {
        match &self.source {
            InteractionUser::Guild(GuildUser { member, .. }) => &member.user,
            InteractionUser::Dm(DmUser { user }) => user,
        }
    }

    pub fn member(&self) -> Option<&GuildMember> {
        match &self.source {
            InteractionUser::Guild(GuildUser { member, .. }) => Some(member),
            InteractionUser::Dm(_) => None,
        }
    }
}

// its not actually self, you dumb clippy::nursery
#[allow(clippy::use_self)]
impl<Data: InteractionPayload> InteractionUse<Data, Unused> {
    pub fn new(
        id: InteractionId,
        application_id: ApplicationId,
        data: Data,
        channel: ChannelId,
        source: InteractionUser,
        token: Token,
    ) -> Self {
        Self {
            id,
            application_id,
            data,
            channel,
            source,
            token,
            _priv: PhantomData,
        }
    }

    pub async fn respond<Client, Message>(self, client: Client, message: Message) -> ClientResult<InteractionUse<Data, Used>>
        where Client: AsRef<DiscordClient> + Send,
              Message: Into<InteractionMessage> + Send,
    {
        let client = client.as_ref();
        client.create_interaction_response(
            self.id,
            self.token.clone(),
            InteractionResponse::ChannelMessageWithSource(message.into()),
        ).await.map(|_| self.into())
    }

    pub async fn defer<Client: AsRef<DiscordClient> + Send>(self, client: Client) -> ClientResult<InteractionUse<Data, Deferred>> {
        let client = client.as_ref();
        client.create_interaction_response(
            self.id,
            self.token.clone(),
            InteractionResponse::DeferredChannelMessageWithSource,
        ).await.map(|_| self.into())
    }

    pub async fn delete<B, State>(self, state: State) -> ClientResult<InteractionUse<Data, Used>>
        where B: Send + Sync + 'static,
              State: AsRef<BotState<B>> + Send,
    {
        let client = state.as_ref();
        self.defer(client).await?.delete(&client).await
    }
}

impl<Data: InteractionPayload> InteractionUse<Data, Used> {
    pub async fn edit<B, State, Message>(&mut self, state: State, message: Message) -> ClientResult<()>
        where B: Send + Sync + 'static,
              State: AsRef<BotState<B>> + Send + Sync,
              Message: Into<InteractionMessage> + Send,
    {
        let state = state.as_ref();
        state.client.edit_interaction_response(
            state.application_id(),
            self.token.clone(),
            message.into(),
        ).await?;
        Ok(())
    }

    #[allow(dead_code)]
    pub async fn delete<B, State>(self, state: State) -> ClientResult<()>
        where B: Send + Sync + 'static,
              State: AsRef<BotState<B>> + Send + Sync
    {
        let state = state.as_ref();
        state.client.delete_interaction_response(
            state.application_id(),
            self.token,
        ).await
    }

    pub async fn followup<B, State, Message>(&self, state: State, message: Message) -> ClientResult<crate::model::message::Message>
        where B: Send + Sync + 'static,
              State: AsRef<BotState<B>> + Send + Sync,
              Message: Into<WebhookMessage> + Send,
    {
        let state = state.as_ref();
        state.client.create_followup_message(
            state.application_id(),
            self.token.clone(),
            message.into(),
        ).await
    }
}

#[allow(clippy::use_self)]
impl<Data: InteractionPayload> InteractionUse<Data, Deferred> {
    pub async fn followup<B, State, Message>(&self, state: State, message: Message) -> ClientResult<crate::model::message::Message>
        where B: Send + Sync + 'static,
              State: AsRef<BotState<B>> + Send + Sync,
              Message: Into<WebhookMessage> + Send,
    {
        let state = state.as_ref();
        state.client.create_followup_message(
            state.application_id(),
            self.token.clone(),
            message.into(),
        ).await
    }

    pub async fn edit<B, State, Message>(self, state: State, message: Message) -> ClientResult<InteractionUse<Data, Used>>
        where B: Send + Sync + 'static,
              State: AsRef<BotState<B>> + Send + Sync,
              Message: Into<InteractionMessage> + Send,
    {
        let state = state.as_ref();
        state.client.edit_interaction_response(
            state.application_id(),
            self.token.clone(),
            message.into(),
        ).await?;
        Ok(self.into())
    }

    pub async fn delete<B, State>(self, state: State) -> ClientResult<InteractionUse<Data, Used>>
        where B: Send + Sync + 'static,
              State: AsRef<BotState<B>> + Send + Sync
    {
        let state = state.as_ref();
        state.client.delete_interaction_response(
            state.application_id(),
            self.token.clone(),
        ).await?;
        Ok(self.into())
    }
}

impl<C: ComponentData, U: Usability> InteractionUse<C, U>
    where InteractionUse<C, Used>: From<InteractionUse<C, U>>,
{
    pub async fn update<Client, Message>(self, client: Client, message: Message) -> ClientResult<InteractionUse<C, Used>>
        where Client: AsRef<DiscordClient> + Send,
              Message: Into<InteractionMessage> + Send,
    {
        let client = client.as_ref();
        client.create_interaction_response(
            self.id,
            self.token.clone(),
            InteractionResponse::UpdateMessage(message.into()),
        ).await.map(|_| self.into())
    }

    pub async fn defer_update<Client>(self, client: Client) -> ClientResult<InteractionUse<C, Used>>
        where Client: AsRef<DiscordClient> + Send,
    {
        let client = client.as_ref();
        client.create_interaction_response(
            self.id,
            self.token.clone(),
            InteractionResponse::DeferredUpdateMessage,
        ).await.map(|_| self.into())
    }
}

impl<Data: InteractionPayload + Sync, U: NotUnused + Sync> InteractionUse<Data, U> {
    pub async fn get_message(
        &self,
        cache: &Cache,
        period: Duration,
        timeout: Duration,
    ) -> Option<Message> {
        let start = Instant::now();
        let mut interval = tokio::time::interval(period);
        loop {
            let now = interval.tick().await;
            if let Some(message) = cache.interaction_response(self).await {
                println!("DONE: {:?}", now - start);
                break Some(message);
            }
            log::info!("MISSED ONE = {:?}", now - start);
            if now - start > timeout {
                break None;
            }
        }
    }
}

// impl<Data: InteractionPayload, Use: Usability> From<InteractionUse<Data, Use>> for InteractionUse<Data, Used> {
//     fn from(InteractionUse { id, application_id, data, channel, source, token, _priv }: InteractionUse<Data, Use>) -> Self {
//         Self { id, application_id, data, channel, source, token, _priv: PhantomData }
//     }
// }

#[allow(clippy::use_self)]
impl<Data: InteractionPayload> From<InteractionUse<Data, Unused>> for InteractionUse<Data, Used> {
    fn from(InteractionUse { id, application_id, data, channel, source, token, _priv }: InteractionUse<Data, Unused>) -> Self {
        Self { id, application_id, data, channel, source, token, _priv: PhantomData }
    }
}

#[allow(clippy::use_self)]
impl<Data: InteractionPayload> From<InteractionUse<Data, Unused>> for InteractionUse<Data, Deferred> {
    fn from(InteractionUse { id, application_id, data, channel, source, token, _priv }: InteractionUse<Data, Unused>) -> Self {
        Self { id, application_id, data, channel, source, token, _priv: PhantomData }
    }
}

#[allow(clippy::use_self)]
impl<Data: InteractionPayload> From<InteractionUse<Data, Deferred>> for InteractionUse<Data, Used> {
    fn from(InteractionUse { id, application_id, data, channel, source, token, _priv }: InteractionUse<Data, Deferred>) -> Self {
        Self { id, application_id, data, channel, source, token, _priv: PhantomData }
    }
}

// begin magic happy traits that let the proc macros be epic

macro_rules! option_primitives {
    ($($ty:ty, $variant:ident, $choice:ty);+ $(;)?) => {
        $(
            #[allow(clippy::use_self)]
            impl<C: SlashCommandRaw> CommandData<C> for $ty {
                type Options = InteractionDataOption;

                fn from_options(option: Self::Options) -> Result<Self, CommandParseError> {
                    match option {
                        InteractionDataOption::$variant(
                            DataOption {
                                data: HasValue { value },
                                ..
                            }
                        ) => Ok(value),
                        bad => Err(CommandParseError::BadType(OptionTypeError {
                            value: bad,
                            desired: CommandOptionTypeParsed::String,
                        }))
                    }
                }

                type VecArg = CommandDataOption;

                fn make_args(_: &C) -> Vec<Self::VecArg> {
                    unreachable!()
                }

                // for the primitives, the ChoicePrimitive is the Choice
                type Choice = $choice;
                type ChoicePrimitive = $choice;
            }

            impl OptionCtor for $ty {
                type Data = Self;
                const ARG_NAME: &'static str = stringify!($variant);

                fn option_ctor(data: OptionData<Self::Data>) -> CommandDataOption {
                    CommandDataOption::$variant(data)
                }
            }
        )+
    };
}
option_primitives! {
    String,        String,      String;
    i64,           Integer,     i64;
    bool,          Boolean,     std::convert::Infallible;
    UserId,        User,        std::convert::Infallible;
    ChannelId,     Channel,     std::convert::Infallible;
    RoleId,        Role,        std::convert::Infallible;
    MentionableId, Mentionable, std::convert::Infallible;
    f64,           Number,      f64;
    Attachment,    Attachment,  std::convert::Infallible;
}

macro_rules! option_integers {
    ($($ty:ty, $parsed_type:ident);+ $(;)?) => {
        $(
            #[allow(clippy::use_self)]
            impl<C: SlashCommandRaw> CommandData<C> for $ty {
                type Options = InteractionDataOption;

                fn from_options(option: Self::Options) -> Result<Self, CommandParseError> {
                    use std::convert::TryInto;
                    match option {
                        InteractionDataOption::Integer(
                            DataOption {
                                data: HasValue { value },
                                ..
                            }
                        ) => value.try_into()
                            .map_err(|_| todo!()),
                        bad => Err(CommandParseError::BadType(OptionTypeError {
                            value: bad,
                            desired: CommandOptionTypeParsed::I64,
                        }))
                    }
                }

                type VecArg = CommandDataOption;

                fn make_args(_: &C) -> Vec<Self::VecArg> {
                    unreachable!()
                }

                type Choice = Self;
                type ChoicePrimitive = i64;
            }

            impl OptionCtor for $ty {
                type Data = i64;
                // todo specify positive integer?
                const ARG_NAME: &'static str = "int";

                fn option_ctor(data: OptionData<Self::Data>) -> CommandDataOption {
                    CommandDataOption::Integer(data)
                }
            }
        )+
    };
}
option_integers! {
    usize, Usize;
    u64, U64;
}

macro_rules! option_ids {
    ($($id:ty, $cotp:ident, $name:literal);+ $(;)?) => {
        $(
            impl<C: SlashCommandRaw> CommandData<C> for $id {
                type Options = InteractionDataOption;

                fn from_options(options: Self::Options) -> Result<Self, CommandParseError> {
                    todo!()
                    // options.lower.string()
                    //     .and_then(|s| s.parse().map_err(|_| $crate::errors::OptionTypeError {
                    //         value: $crate::model::old_interaction::OptionValue::String(s),
                    //         desired: CommandOptionTypeParsed::$cotp,
                    //     }))
                    //     .map_err(|e| e.into())
                }

                type VecArg = CommandDataOption;

                fn make_args(_: &C) -> Vec<Self::VecArg> {
                    unreachable!()
                }

                type Choice = Self;
                type ChoicePrimitive = String;
            }

            impl OptionCtor for $id {
                type Data = String;
                const ARG_NAME: &'static str = $name;

                fn option_ctor(cdo: OptionData<Self::Data>) -> CommandDataOption {
                    CommandDataOption::String(cdo)
                }
            }
        )+
    };
}
option_ids! {
    MessageId, MessageId, "message";
    GuildId, GuildId, "guild";
}

pub trait OptionCtor {
    type Data: OptionType;

    /// Get the name of this for generic types that implement [`CommandData`]
    const ARG_NAME: &'static str;

    fn option_ctor(data: OptionData<Self::Data>) -> CommandDataOption;
}

impl<T: OptionCtor<Data=T> + OptionType> OptionCtor for Option<T> {
    type Data = T;

    const ARG_NAME: &'static str = T::ARG_NAME;

    fn option_ctor(data: OptionData<Self::Data>) -> CommandDataOption {
        T::option_ctor(data)
    }
}

// traits to let enums figure out how to impl CommandData
pub enum Highest {}

pub enum Lowest {}

pub trait VecArgLadder: Sized {
    type Raise: VecArgLadder;
    type Lower: VecArgLadder;
    fn wrap(vec: Vec<Self>) -> Vec<CommandOption>;
    fn make<N, D>(name: N, desc: D, lower_options: Vec<Self::Lower>) -> Self
        where N: Into<Cow<'static, str>>,
              D: Into<Cow<'static, str>>;
}

impl VecArgLadder for Infallible {
    type Raise = Self;
    type Lower = Self;

    fn wrap(_: Vec<Self>) -> Vec<CommandOption> {
        unreachable!()
    }

    fn make<N, D>(_: N, _: D, _: Vec<Self::Lower>) -> Self where N: Into<Cow<'static, str>>, D: Into<Cow<'static, str>> {
        unreachable!()
    }
}

impl VecArgLadder for Highest {
    type Raise = Self;
    // todo should maybe just be self?
    type Lower = SubCommandGroupOption;

    fn wrap(_: Vec<Self>) -> Vec<CommandOption> {
        unreachable!("should never have a `Highest`")
    }

    fn make<N, D>(_: N, _: D, _: Vec<Self::Lower>) -> Self
        where N: Into<Cow<'static, str>>,
              D: Into<Cow<'static, str>> {
        unreachable!("should never have a `Highest`")
    }
}

impl VecArgLadder for SubCommandGroupOption {
    type Raise = Highest;
    type Lower = SubCommandOption;

    fn wrap(vec: Vec<Self>) -> Vec<CommandOption> {
        vec.into_iter()
            .map(Self::into)
            .collect()
    }

    fn make<N, D>(name: N, desc: D, lower_options: Vec<Self::Lower>) -> Self
        where N: Into<Cow<'static, str>>,
              D: Into<Cow<'static, str>> {
        Self::SubCommandGroup(OptionData {
            name: name.into(),
            name_localizations: Default::default(),
            description: desc.into(),
            description_localizations: Default::default(),
            extra_data: command::SubCommandGroup {
                sub_commands: lower_options,
            },
        })
    }
}

impl VecArgLadder for SubCommandOption {
    type Raise = SubCommandGroupOption;
    type Lower = CommandDataOption;

    fn wrap(vec: Vec<Self>) -> Vec<CommandOption> {
        vec.into_iter()
            .map(Self::into)
            .collect()
    }

    fn make<N, D>(name: N, desc: D, lower_options: Vec<Self::Lower>) -> Self
        where N: Into<Cow<'static, str>>,
              D: Into<Cow<'static, str>> {
        Self::SubCommand(OptionData {
            name: name.into(),
            name_localizations: Default::default(),
            description: desc.into(),
            description_localizations: Default::default(),
            extra_data: command::SubCommand {
                data_options: lower_options,
            },
        })
    }
}

impl VecArgLadder for CommandDataOption {
    type Raise = SubCommandOption;
    type Lower = Lowest;

    fn wrap(vec: Vec<Self>) -> Vec<CommandOption> {
        vec.into_iter()
            .map(Self::into)
            .collect()
    }

    fn make<N, D>(_: N, _: D, _: Vec<Self::Lower>) -> Self
        where N: Into<Cow<'static, str>>,
              D: Into<Cow<'static, str>> {
        // Self::String(CommandDataOption::new(name, desc))
        unimplemented!("this should be covered by the proc-macro for structs?")
    }
}

impl VecArgLadder for Lowest {
    // todo should maybe be Self?
    type Raise = CommandDataOption;
    type Lower = Self;

    fn wrap(_: Vec<Self>) -> Vec<CommandOption> {
        unreachable!("should never have a `Lowest`")
    }

    fn make<N, D>(_: N, _: D, _: Vec<Self::Lower>) -> Self
        where N: Into<Cow<'static, str>>,
              D: Into<Cow<'static, str>> {
        unreachable!("should never have a `Lowest`")
    }
}

impl VecArgLadder for () {
    type Raise = ();
    type Lower = ();

    fn wrap(vec: Vec<Self>) -> Vec<CommandOption> {
        // todo ig
        assert!(vec.is_empty());
        Vec::new()
    }

    fn make<N, D>(_: N, _: D, _: Vec<Self::Lower>) -> Self
        where N: Into<Cow<'static, str>>,
              D: Into<Cow<'static, str>> {
        unimplemented!()
    }
}

pub trait OptionsLadder: Sized {
    type Raise: OptionsLadder;
    type Lower: OptionsLadder;

    fn from_data_option(data: InteractionOption) -> Result<Self, CommandParseError>;
}

impl OptionsLadder for Infallible {
    type Raise = Self;
    type Lower = Self;

    fn from_data_option(_: InteractionOption) -> Result<Self, CommandParseError> {
        unreachable!()
    }
}

impl OptionsLadder for Highest {
    // todo should maybe just be self?
    type Raise = Self;
    type Lower = InteractionOption;

    fn from_data_option(_: InteractionOption) -> Result<Self, CommandParseError> {
        unreachable!("should never have a `Highest`")
    }
}

impl OptionsLadder for InteractionOption {
    type Raise = Highest;
    type Lower = DataOption<SubCommandGroup>;

    fn from_data_option(data: InteractionOption) -> Result<Self, CommandParseError> {
        Ok(data)
    }
}

impl OptionsLadder for DataOption<SubCommandGroup> {
    type Raise = InteractionOption;
    type Lower = DataOption<SubCommand>;

    fn from_data_option(data: InteractionOption) -> Result<Self, CommandParseError> {
        match data {
            InteractionOption::Group(group) => Ok(group),
            InteractionOption::Command(_) => Err(CommandParseError::BadCommandOccurrence),
            InteractionOption::Values(_) => Err(CommandParseError::BadGroupOccurrence),
            // // todo more error types
            // InteractionOption::Empty => Err(CommandParseError::BadGroupOccurrence)
        }
    }
}

impl OptionsLadder for DataOption<SubCommand> {
    type Raise = DataOption<SubCommandGroup>;
    type Lower = Vec<InteractionDataOption>;

    fn from_data_option(data: InteractionOption) -> Result<Self, CommandParseError> {
        match data {
            InteractionOption::Group(_) => Err(CommandParseError::BadGroupOccurrence),
            InteractionOption::Command(data) => Ok(data),
            // todo better errors
            InteractionOption::Values(_) => Err(CommandParseError::BadGroupOccurrence),
            // InteractionOption::Empty => Err(CommandParseError::BadCommandOccurrence),
        }
    }
}

impl OptionsLadder for Vec<InteractionDataOption> {
    type Raise = DataOption<SubCommand>;
    type Lower = InteractionDataOption;

    fn from_data_option(data: InteractionOption) -> Result<Self, CommandParseError> {
        match data {
            InteractionOption::Group(_) => Err(CommandParseError::BadGroupOccurrence),
            InteractionOption::Command(_) => Err(CommandParseError::BadCommandOccurrence),
            InteractionOption::Values(vec) => Ok(vec),
            // InteractionOption::Empty => Ok(Vec::new()),
        }
    }
}

impl OptionsLadder for InteractionDataOption {
    type Raise = Vec<InteractionDataOption>;
    type Lower = Lowest;

    fn from_data_option(data: InteractionOption) -> Result<Self, CommandParseError> {
        unreachable!()
        // println!("fdo: O");
        // match data {
        //     InteractionOption::Command(_) => Err(CommandParseError::BadGroupOccurrence),
        //     InteractionOption::Group(_) => Err(CommandParseError::BadCommandOccurrence),
        //     InteractionOption::Values(mut values) => {
        //         warn!("This probably shouldn't be happening???");
        //         warn!("values = {:?}", values);
        //         Ok(values.remove(0))
        //     }
        // }
    }
}

impl OptionsLadder for Lowest {
    // todo should just be self?
    type Raise = InteractionDataOption;
    type Lower = Self;

    fn from_data_option(_: InteractionOption) -> Result<Self, CommandParseError> {
        unreachable!("should never have a `Lowest`")
    }
}

impl OptionsLadder for () {
    type Raise = ();
    type Lower = ();

    fn from_data_option(_: InteractionOption) -> Result<Self, CommandParseError> {
        Ok(())
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum VarargState {
    Fixed(usize),
    Variable,
    None,
}

impl VarargState {
    pub fn number(self) -> Option<usize> {
        match self {
            Self::Fixed(n) => Some(n),
            Self::Variable => None,
            Self::None => panic!("Tried to get the number of varargs for a non-vararg option"),
        }
    }
}

/// the big boi himself
pub trait CommandData<Command: SlashCommandRaw>: Sized {
    type Options: OptionsLadder + Send;
    /// function to go from (the options in a) `InteractionData` -> Self
    fn from_options(options: Self::Options) -> Result<Self, CommandParseError>;

    type VecArg: VecArgLadder;
    // todo: VecArg *maybe* should have the Vec<> on it, so that this can just return one?
    //  do I ever actually return vec![one] or just do I always panic?
    /// functionality to got from Self -> Command for sending to Discord
    fn make_args(command: &Command) -> Vec<Self::VecArg>;

    type Choice;
    // fn make_choices() -> Vec<Choice<Self::Choice>> {
    //     unreachable!("I think this is still true")
    // }
    fn make_choices() -> Vec<Self::Choice> {
        Vec::new()
    }
    type ChoicePrimitive;
    fn into_command_choice(self) -> Choice<Self::ChoicePrimitive> {
        unreachable!()
    }
    fn vararg_number() -> VarargState { VarargState::None }
}

impl<C: SlashCommandRaw> CommandData<C> for Infallible {
    type Options = Self;

    fn from_options(_: Self::Options) -> Result<Self, CommandParseError> {
        unreachable!()
    }

    type VecArg = Self;

    fn make_args(_: &C) -> Vec<Self::VecArg> {
        unreachable!()
    }

    type Choice = Self;
    type ChoicePrimitive = Self;
}

// let `()` be used for commands with no options
impl<Command: SlashCommandRaw> CommandData<Command> for () {
    type Options = Vec<interaction::InteractionDataOption>;

    fn from_options(_: Self::Options) -> Result<Self, CommandParseError> {
        Ok(())
    }

    type VecArg = ();

    fn make_args(_: &Command) -> Vec<Self::VecArg> {
        Vec::new()
    }

    type Choice = Infallible;
    type ChoicePrimitive = Infallible;
}

// impl for some containers
impl<C: SlashCommandRaw, T: CommandData<C>> CommandData<C> for Option<T> {
    type Options = T::Options;

    fn from_options(data: Self::Options) -> Result<Self, CommandParseError> {
        // `T::from_data` failing means that the data was the wrong type, not that it was absent
        // Absent data is handled before calling this function
        Ok(Some(T::from_options(data)?))
    }

    type VecArg = T::VecArg;

    fn make_args(command: &C) -> Vec<Self::VecArg> {
        T::make_args(command)
    }

    type Choice = T::Choice;
    type ChoicePrimitive = T::ChoicePrimitive;
}

impl<T, C, S> CommandData<C> for HashSet<T, S>
    where
        T: CommandData<C, VecArg=command::CommandDataOption, Options=interaction::InteractionDataOption> + Eq + Hash,
        C: SlashCommandRaw,
        S: BuildHasher + Default,
{
    type Options = Vec<interaction::InteractionDataOption>;

    fn from_options(options: Self::Options) -> Result<Self, CommandParseError> {
        options.into_iter().map(T::from_options).collect()
    }

    type VecArg = command::CommandDataOption;

    fn make_args(c: &C) -> Vec<Self::VecArg> {
        T::make_args(c)
    }

    type Choice = T::Choice;
    fn make_choices() -> Vec<Self::Choice> {
        T::make_choices()
    }

    type ChoicePrimitive = T::ChoicePrimitive;

    fn vararg_number() -> VarargState {
        VarargState::Variable
    }
}

impl<T, C> CommandData<C> for BTreeSet<T>
    where
        T: CommandData<C, VecArg=command::CommandDataOption, Options=interaction::InteractionDataOption> + Ord,
        C: SlashCommandRaw,
{
    type Options = Vec<interaction::InteractionDataOption>;

    fn from_options(options: Self::Options) -> Result<Self, CommandParseError> {
        options.into_iter().map(T::from_options).collect()
    }

    type VecArg = command::CommandDataOption;

    fn make_args(c: &C) -> Vec<Self::VecArg> {
        T::make_args(c)
    }

    type Choice = T::Choice;
    fn make_choices() -> Vec<Self::Choice> {
        T::make_choices()
    }

    type ChoicePrimitive = T::ChoicePrimitive;

    fn vararg_number() -> VarargState {
        VarargState::Variable
    }
}

#[allow(clippy::use_self)]
impl<T, C> CommandData<C> for Vec<T>
    where
        T: CommandData<C, VecArg=command::CommandDataOption, Options=interaction::InteractionDataOption>,
        C: SlashCommandRaw,
{
    type Options = Vec<interaction::InteractionDataOption>;

    fn from_options(options: Self::Options) -> Result<Self, CommandParseError> {
        options.into_iter().map(T::from_options).collect()
    }

    type VecArg = command::CommandDataOption;

    fn make_args(c: &C) -> Vec<Self::VecArg> {
        T::make_args(c)
    }

    type Choice = T::Choice;
    fn make_choices() -> Vec<Self::Choice> {
        T::make_choices()
    }

    type ChoicePrimitive = T::ChoicePrimitive;

    fn vararg_number() -> VarargState {
        VarargState::Variable
    }
}

impl<T, C, const N: usize> CommandData<C> for [T; N]
    where
        T: CommandData<C, VecArg=command::CommandDataOption, Options=interaction::InteractionDataOption>,
        C: SlashCommandRaw,
{
    type Options = Vec<interaction::InteractionDataOption>;

    fn from_options(options: Self::Options) -> Result<Self, CommandParseError> {
        let iter = options.into_iter().map(T::from_options);
        utils::array_try_from_iter(iter, |i| CommandParseError::MissingOption(
            format!("Didn't have option number {}", i + 1)
        ))
    }

    type VecArg = command::CommandDataOption;

    fn make_args(command: &C) -> Vec<Self::VecArg> {
        T::make_args(command)
    }

    type Choice = T::Choice;
    fn make_choices() -> Vec<Self::Choice> {
        T::make_choices()
    }

    type ChoicePrimitive = T::ChoicePrimitive;

    fn vararg_number() -> VarargState {
        VarargState::Fixed(N)
    }
}

pub trait MenuData: Sized + FromStr {
    type Data: SelectMenuType;

    fn into_option(self) -> <Self::Data as SelectMenuType>::SelectOption;
    fn all() -> Vec<Self>;
    fn options() -> Vec<<Self::Data as SelectMenuType>::SelectOption>;
}
macro_rules! id_menu {
    ($($id:ty),+ $(,)?) => {
        $(
            impl MenuData for $id {
                type Data = Self;
                fn into_option(self) -> () { unreachable!() }
                fn all() -> Vec<Self> { Vec::new() }
                fn options() -> Vec<()> { Vec::new() }
            }
        )+
    };
}
id_menu!(UserId, RoleId, MentionableId, ChannelId);
impl MenuData for String {
    type Data = Self;

    fn into_option(self) -> SelectOption {
        SelectOption {
            label: self.clone(),
            value: self,
            description: None,
            emoji: None,
            default: false,
        }
    }
    fn all() -> Vec<Self> {
        Vec::new()
    }
    fn options() -> Vec<SelectOption> {
        Vec::new()
    }
}
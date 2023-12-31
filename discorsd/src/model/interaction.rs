use std::collections::{HashMap, HashSet};
use std::fmt::{self, Display, Formatter};
use std::iter;
use std::vec::IntoIter;

use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Deserializer};
use serde_derive::{Deserialize, Serialize};

use crate::IdMap;
use crate::model::channel::ChannelType;
use crate::model::command::{CommandOptionType, CommandType};
use crate::model::components::{ComponentId, ComponentType};
use crate::model::guild::GuildMember;
use crate::model::ids::*;
use crate::model::locales::Locale;
use crate::model::message::{Attachment, Message};
use crate::model::permissions::{Permissions, Role};
use crate::model::user::User;
use crate::serde_utils::null_as_default;

serde_num_tag! { just Deserialize =>
    /// hi
    #[derive(Debug, Clone)]
    pub enum Interaction = "type": u8 as InteractionType {
        (1) = Ping,
        (2) = ApplicationCommand(InteractionData<ApplicationCommandData>),
        (3) = MessageComponent(InteractionData<MessageComponentData>),
        (4) = ApplicationCommandAutocomplete(InteractionData<ApplicationCommandData>),
        (5) = ModalSubmit(InteractionData<ModalSubmitData>)
    }
}

serde_num_tag! { just Deserialize =>
    #[derive(Debug, Clone)]
    pub enum ApplicationCommandData = "type": CommandType {
        (CommandType::SlashCommand) = SlashCommand {
            id: CommandId,
            name: String,
            #serde = default
            options: InteractionOption,
        },
        (CommandType::UserCommand) = UserCommand {
            id: CommandId,
            name: String,
            target_id: UserId,
            resolved: ResolvedData,
        },
        (CommandType::MessageCommand) = MessageCommand {
            id: CommandId,
            name: String,
            target_id: MessageId,
            resolved: ResolvedData,
        },
    }
}

impl Id for ApplicationCommandData {
    type Id = CommandId;

    fn id(&self) -> Self::Id {
        match *self {
            Self::SlashCommand { id, .. } => id,
            Self::UserCommand { id, .. } => id,
            Self::MessageCommand { id, .. } => id,
        }
    }
}

impl PartialEq for ApplicationCommandData {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

impl ApplicationCommandData {
    pub fn name(&self) -> &str {
        match self {
            Self::SlashCommand { name, .. } => name,
            Self::UserCommand { name, .. } => name,
            Self::MessageCommand { name, .. } => name,
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct Token(pub String);

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct InteractionData<Data> {
    /// ID of the interaction
    pub id: InteractionId,
    /// ID of the application this interaction is for
    pub application_id: ApplicationId,
    /// Continuation token for responding to the interaction
    pub token: Token,
    /// Channel that the interaction was sent from
    pub channel_id: ChannelId,
    // /// partial Channel that the interaction was sent from
    // // todo partial channel
    // pub channel: Option<partial channel object>,
    /// Interaction data payload
    pub data: Data,
    /// For components, the message they were attached to
    // todo figure out a good way to have this only be included for MessageComponent type could put
    //  it in Data and #flatten `data`, but then ApplicationCommandData and ModalSubmitData would
    //  have to be nested another level too I think
    pub message: Option<Message>,
    #[serde(flatten)]
    pub user: InteractionUser,
    /// Bitwise set of permissions the app or bot has within the channel the interaction was sent from
    pub app_permissions: Option<Permissions>,
    /// Selected language of the invoking user
    // todo this might be optional or it might just be for ApplicationCommandData
    pub locale: Option<Locale>,
}

// todo this seems weird
impl Id for InteractionData<ApplicationCommandData> {
    type Id = CommandId;

    fn id(&self) -> Self::Id {
        self.data.id()
    }
}

impl PartialEq for InteractionData<ApplicationCommandData> {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id()
    }
}

/// Information about the guild and guild member that invoked this interaction
#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct GuildUser {
    /// The guild the interaction was sent from
    #[serde(rename = "guild_id")]
    pub id: GuildId,
    /// Guild member data for the invoking user
    pub member: GuildMember,
    /// Guild's preferred locale
    #[serde(rename = "guild_locale")]
    pub locale: Option<Locale>,
}

/// Information about the user that invoked this interaction
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DmUser {
    /// The user that invoked this interaction
    pub user: User,
}

/// Information about where this interaction occurred, whether in a guild channel or in a dm
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum InteractionUser {
    /// This interaction was sent in a guild, see [GuildUser](GuildUser)
    Guild(GuildUser),
    /// This interaction was sent in a dm, see [DmUser](DmUser)
    Dm(DmUser),
}

// for Error usage
impl Display for InteractionUser {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for InteractionUser {}

impl InteractionUser {
    /// Returns the [GuildUser](GuildUser) that used this command, if it was invoked in a guild
    pub fn guild(self) -> Option<GuildUser> {
        match self {
            Self::Guild(gs) => Some(gs),
            Self::Dm(_) => None,
        }
    }
    /// Returns the [GuildUser](GuildUser) that used this command, if it was invoked in a guild
    pub fn guild_ref(&self) -> Option<&GuildUser> {
        match self {
            Self::Guild(g) => Some(g),
            Self::Dm(_) => None,
        }
    }
    /// Returns the [User](User) that used this command, if it was invoked in a guild
    pub fn user(self) -> Option<User> {
        match self {
            Self::Guild(_) => None,
            Self::Dm(DmUser { user }) => Some(user),
        }
    }
    /// Returns the [User](User) that used this command, if it was invoked in a guild
    pub fn user_ref(&self) -> Option<&User> {
        match self {
            Self::Guild(_) => None,
            Self::Dm(DmUser { user }) => Some(user),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(try_from = "Vec<InteractionOptionRaw>")]
pub enum InteractionOption {
    Command(DataOption<SubCommand>),
    Group(DataOption<SubCommandGroup>),
    Values(Vec<InteractionDataOption>),
}

impl Default for InteractionOption {
    fn default() -> Self {
        Self::Values(Vec::new())
    }
}

impl TryFrom<Vec<InteractionOptionRaw>> for InteractionOption {
    // todo
    type Error = &'static str;

    fn try_from(value: Vec<InteractionOptionRaw>) -> Result<Self, Self::Error> {
        println!("value = {:?}", value);
        fn values(first: InteractionDataOption, rest: IntoIter<InteractionOptionRaw>) -> Result<InteractionOption, &'static str> {
            let vec = iter::once(Ok(first))
                .chain(rest.map(|value| match value {
                    InteractionOptionRaw::SubCommand(_) => Err("bad sc"),
                    InteractionOptionRaw::SubCommandGroup(_) => Err("bad group"),
                    InteractionOptionRaw::String(d) => Ok(InteractionDataOption::String(d)),
                    InteractionOptionRaw::Integer(d) => Ok(InteractionDataOption::Integer(d)),
                    InteractionOptionRaw::Boolean(d) => Ok(InteractionDataOption::Boolean(d)),
                    InteractionOptionRaw::User(d) => Ok(InteractionDataOption::User(d)),
                    InteractionOptionRaw::Channel(d) => Ok(InteractionDataOption::Channel(d)),
                    InteractionOptionRaw::Role(d) => Ok(InteractionDataOption::Role(d)),
                    InteractionOptionRaw::Mentionable(d) => Ok(InteractionDataOption::Mentionable(d)),
                    InteractionOptionRaw::Number(d) => Ok(InteractionDataOption::Number(d)),
                    InteractionOptionRaw::Attachment(d) => Ok(InteractionDataOption::Attachment(d)),
                }))
                .try_collect()?;
            Ok(InteractionOption::Values(vec))
        }
        let mut rest = value.into_iter();
        match rest.next().ok_or("no opts")? {
            InteractionOptionRaw::SubCommand(c) => Ok(Self::Command(c)),
            InteractionOptionRaw::SubCommandGroup(g) => Ok(Self::Group(g)),
            InteractionOptionRaw::String(d) => values(InteractionDataOption::String(d), rest),
            InteractionOptionRaw::Integer(d) => values(InteractionDataOption::Integer(d), rest),
            InteractionOptionRaw::Boolean(d) => values(InteractionDataOption::Boolean(d), rest),
            InteractionOptionRaw::User(d) => values(InteractionDataOption::User(d), rest),
            InteractionOptionRaw::Channel(d) => values(InteractionDataOption::Channel(d), rest),
            InteractionOptionRaw::Role(d) => values(InteractionDataOption::Role(d), rest),
            InteractionOptionRaw::Mentionable(d) => values(InteractionDataOption::Mentionable(d), rest),
            InteractionOptionRaw::Number(d) => values(InteractionDataOption::Number(d), rest),
            InteractionOptionRaw::Attachment(d) => values(InteractionDataOption::Attachment(d), rest),
        }
    }
}

serde_num_tag! { just Deserialize =>
    // old::InteractionDataOption
    #[derive(Debug, Clone)]
    pub enum InteractionOptionRaw = "type": CommandOptionType {
        (CommandOptionType::SubCommand) = SubCommand(DataOption<SubCommand>),
        (CommandOptionType::SubCommandGroup) = SubCommandGroup(DataOption<SubCommandGroup>),
        (CommandOptionType::String) = String(DataOption<String>),
        /// Any integer between -2^53 and 2^53
        (CommandOptionType::Integer) = Integer(DataOption<i64>),
        (CommandOptionType::Boolean) = Boolean(DataOption<bool>),
        (CommandOptionType::User) = User(DataOption<UserId>),
        /// Includes all channel types + categories
        (CommandOptionType::Channel) = Channel(DataOption<ChannelId>),
        (CommandOptionType::Role) = Role(DataOption<RoleId>),
        /// Includes users and roles
        (CommandOptionType::Mentionable) = Mentionable(DataOption<MentionableId>),
        /// Any double between -2^53 and 2^53
        (CommandOptionType::Number) = Number(DataOption<f64>),
        /// attachment object
        (CommandOptionType::Attachment) = Attachment(DataOption<Attachment>)
    }
}

serde_num_tag! { just Deserialize =>
    // old::ValueOption
    #[derive(Debug, Clone)]
    pub enum InteractionDataOption = "type": CommandOptionType {
        (CommandOptionType::String) = String(DataOption<String>),
        /// Any integer between -2^53 and 2^53
        (CommandOptionType::Integer) = Integer(DataOption<i64>),
        (CommandOptionType::Boolean) = Boolean(DataOption<bool>),
        (CommandOptionType::User) = User(DataOption<UserId>),
        /// Includes all channel types + categories
        (CommandOptionType::Channel) = Channel(DataOption<ChannelId>),
        (CommandOptionType::Role) = Role(DataOption<RoleId>),
        /// Includes users and roles
        (CommandOptionType::Mentionable) = Mentionable(DataOption<MentionableId>),
        /// Any double between -2^53 and 2^53
        (CommandOptionType::Number) = Number(DataOption<f64>),
        /// attachment object
        (CommandOptionType::Attachment) = Attachment(DataOption<Attachment>)
    }
}

impl InteractionDataOption {
    pub fn name(&self) -> &str {
        match self {
            Self::String(d) => &d.name,
            Self::Integer(d) => &d.name,
            Self::Boolean(d) => &d.name,
            Self::User(d) => &d.name,
            Self::Channel(d) => &d.name,
            Self::Role(d) => &d.name,
            Self::Mentionable(d) => &d.name,
            Self::Number(d) => &d.name,
            Self::Attachment(d) => &d.name,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HasOptions<T> {
    /// Present if this option is a group or subcommand
    pub options: T,
}

mod __priv {
    use super::*;

    #[derive(Deserialize)]
    struct HasOptionShim<T> {
        options: T,
    }

    impl<'de> Deserialize<'de> for HasOptions<Vec<InteractionDataOption>> {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let HasOptionShim { options } = HasOptionShim::deserialize(d)?;
            Ok(Self { options })
        }
    }

    impl<'de> Deserialize<'de> for HasOptions<DataOption<SubCommand>> {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let HasOptionShim { mut options } = HasOptionShim::<Vec<_>>::deserialize(d)?;
            Ok(Self { options: options.remove(0) })
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct HasValue<T> {
    /// Value of the option resulting from user input
    pub value: T,
}

pub trait OptionType {
    type Data;
}

macro_rules! data_type {
    (HasOptions<$opt:ty> => $($t:ty),+ $(,)?) => {
        $(
            impl OptionType for $t { type Data = HasOptions<$opt>; }
        )*
    };
    (HasValue => $($t:ty),+ $(,)?) => {
        $(
            impl OptionType for $t { type Data = HasValue<$t>; }
        )*
    };
    (
        $($($t:ty),+ $(,)? => $has:tt$(<$opt:ty>)? );+ $(;)?
    ) => {
        $( data_type! { $has$(<$opt>)? => $($t),+ } )+
    };
}

data_type! {
    SubCommand => HasOptions<Vec<InteractionDataOption>>;
    SubCommandGroup => HasOptions<DataOption<SubCommand>>;
    String, i64, bool, UserId, ChannelId, RoleId, MentionableId, f64, Attachment => HasValue;
}

#[derive(Debug, Clone)]
pub struct SubCommand {}

#[derive(Debug, Clone)]
pub struct SubCommandGroup {}

#[derive(Deserialize, Debug, Clone)]
pub struct DataOption<T: OptionType> {
    /// 1-32 character name
    pub name: String,
    /// Localization dictionary for name field. Values follow the same restrictions as name
    #[serde(deserialize_with = "null_as_default", default)]
    pub name_localizations: HashMap<Locale, String>,
    /// value or sub options
    #[serde(flatten)]
    pub data: T::Data,
    /// true if this option is the currently focused option for autocomplete
    #[serde(default)]
    pub focused: bool,
}

#[derive(Deserialize, Debug, Default, Clone, PartialEq)]
#[serde(default)]
pub struct ResolvedData {
    /// the ids and User objects
    pub users: IdMap<User>,
    /// the ids and partial Member objects
    pub members: HashMap<UserId, PartialGuildMember>,
    /// the ids and Role objects
    pub roles: IdMap<Role>,
    /// the ids and partial Channel objects
    pub channels: IdMap<PartialChannel>,
    /// the ids and partial Message objects
    // todo what is in a partial message
    pub messages: IdMap<Message>,
    /// the ids and attachment objects
    pub attachments: IdMap<Attachment>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
// todo rename GuildMemberInfo (and flatten in as the data holder in GuildMember)
pub struct PartialGuildMember {
    /// this users guild nickname
    pub nick: Option<String>,
    /// array of role object ids
    pub roles: HashSet<RoleId>,
    /// when the user joined the guild
    pub joined_at: DateTime<Utc>,
    /// when the user started boosting the guild
    pub premium_since: Option<DateTime<Utc>>,
    /// whether the user has passed the guild's Membership Screening requirements
    #[serde(default)]
    pub pending: bool,
}

// todo threads should also have `thread_metadata` and `parent_id`
#[derive(Deserialize, Debug, Clone)]
pub struct PartialChannel {
    /// the id of this channel
    pub id: ChannelId,
    /// the name of the channel (2-100 characters)
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ChannelType,
    /// Permissions for the channel
    pub permissions: Permissions,
}
id_impl!(PartialChannel => ChannelId);

serde_num_tag! { just Deserialize =>
    #[derive(Debug, Clone)]
    // just "type" because ActionRowData is only in modals
    pub enum ActionRowData = "type": ComponentType {
        (ComponentType::ActionRow) = ActionRow {
            components: Vec<MessageComponentData>,
        }
    }
}

serde_num_tag! { just Deserialize =>
    #[derive(Debug, Clone)]
    // "component_type" when inline components, "type" for modals
    pub enum MessageComponentData = "component_type" alias "type": ComponentType {
        (ComponentType::Button) = Button(ButtonPressData),
        (ComponentType::StringMenu) = StringMenu(MenuSelectDataRaw),
        (ComponentType::TextInput) = TextInput(TextSubmitData),
        (ComponentType::UserMenu) = UserMenu(MenuSelectDataRaw),
        (ComponentType::RoleMenu) = RoleMenu(MenuSelectDataRaw),
        (ComponentType::MentionableMenu) = MentionableMenu(MenuSelectDataRaw),
        (ComponentType::ChannelMenu) = ChannelMenu(MenuSelectDataRaw),
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct ButtonPressData {
    /// the custom_id of the component
    pub custom_id: ComponentId,
    #[serde(default)]
    pub resolved: ResolvedData,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct MenuSelectDataRaw {
    /// the custom_id of the component
    pub custom_id: ComponentId,
    /// values the user selected in a select menu component
    /// Takes advantage of String & Id's all being sent by Discord as strings in the json
    pub values: Vec<String>,
    #[serde(default)]
    pub resolved: ResolvedData,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuSelectData {
    /// the custom_id of the component
    pub custom_id: ComponentId,
    pub resolved: ResolvedData,
}

#[derive(Deserialize, Debug, Clone)]
pub struct TextSubmitData {
    /// the values submitted by the user
    pub custom_id: ComponentId,
    pub value: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ModalSubmitData {
    /// the custom_id of the modal
    pub custom_id: ComponentId,
    /// the values submitted by the user
    pub components: Vec<ActionRowData>,
}

#[cfg(test)]
mod tests {
    use crate::model::interaction::{ApplicationCommandData, Interaction};

    #[test]
    fn slash_command() {
        const SLASH_COMMAND_INTERACTION: &str = r#"{
    "type": 2,
    "token": "A_UNIQUE_TOKEN",
    "application_id": "1421512",
    "member": {
        "user": {
            "id": "53908232506183680",
            "username": "Mason",
            "avatar": "a_d5efa99b3eeaa7dd43acca82f5692432",
            "discriminator": "1337",
            "public_flags": 131141
        },
        "roles": ["539082325061836999"],
        "premium_since": null,
        "permissions": "2147483647",
        "pending": false,
        "nick": null,
        "mute": false,
        "joined_at": "2017-03-13T19:19:14.040000+00:00",
        "is_pending": false,
        "deaf": false
    },
    "id": "786008729715212338",
    "guild_id": "290926798626357999",
    "app_permissions": "442368",
    "guild_locale": "en-US",
    "locale": "en-US",
    "data": {
        "options": [{
            "type": 3,
            "name": "cardname",
            "value": "The Gitrog Monster"
        }],
        "type": 1,
        "name": "cardsearch",
        "id": "771825006014889984"
    },
    "channel_id": "645027906669510667"
}"#;
        serde_json::from_str::<Interaction>(SLASH_COMMAND_INTERACTION).unwrap();
    }

    #[test]
    fn message_command() {
        const MESSAGE_COMMAND_INTERACTION: &str = r#"{
    "application_id": "775799577604522054",
    "channel_id": "772908445358620702",
    "data": {
        "id": "866818195033292851",
        "name": "context-menu-message-2",
        "resolved": {
            "messages": {
                "867793854505943041": {
                    "attachments": [],
                    "author": {
                        "avatar": "a_f03401914fb4f3caa9037578ab980920",
                        "discriminator": "6538",
                        "id": "167348773423415296",
                        "public_flags": 1,
                        "username": "ian"
                    },
                    "channel_id": "772908445358620702",
                    "components": [],
                    "content": "some message",
                    "edited_timestamp": null,
                    "embeds": [],
                    "flags": 0,
                    "id": "867793854505943041",
                    "mention_everyone": false,
                    "mention_roles": [],
                    "mentions": [],
                    "pinned": false,
                    "timestamp": "2021-07-22T15:42:57.744000+00:00",
                    "tts": false,
                    "type": 0
                }
            }
        },
        "target_id": "867793854505943041",
        "type": 3
    },
    "guild_id": "772904309264089089",
    "guild_locale": "en-US",
    "app_permissions": "442368",
    "id": "867793873336926249",
    "locale": "en-US",
    "member": {
        "avatar": null,
        "deaf": false,
        "is_pending": false,
        "joined_at": "2020-11-02T20:46:57.364000+00:00",
        "mute": false,
        "nick": null,
        "pending": false,
        "permissions": "274877906943",
        "premium_since": null,
        "roles": ["785609923542777878"],
        "user": {
            "avatar": "a_f03401914fb4f3caa9037578ab980920",
            "discriminator": "6538",
            "id": "167348773423415296",
            "public_flags": 1,
            "username": "ian"
        }
    },
    "token": "UNIQUE_TOKEN",
    "type": 2,
    "version": 1
}"#;
        serde_json::from_str::<Interaction>(MESSAGE_COMMAND_INTERACTION).unwrap();
    }

    #[test]
    fn user_interaction() {
        const USER_COMMAND_INTERACTION: &str = r#"{
    "application_id": "775799577604522054",
    "channel_id": "772908445358620702",
    "data": {
        "id": "866818195033292850",
        "name": "context-menu-user-2",
        "resolved": {
            "members": {
                "809850198683418695": {
                    "avatar": null,
                    "is_pending": false,
                    "joined_at": "2021-02-12T18:25:07.972000+00:00",
                    "nick": null,
                    "pending": false,
                    "permissions": "246997699136",
                    "premium_since": null,
                    "roles": []
                }
            },
            "users": {
                "809850198683418695": {
                    "avatar": "afc428077119df8aabbbd84b0dc90c74",
                    "bot": true,
                    "discriminator": "7302",
                    "id": "809850198683418695",
                    "public_flags": 0,
                    "username": "VoltyDemo"
                }
            }
        },
        "target_id": "809850198683418695",
        "type": 2
    },
    "guild_id": "772904309264089089",
    "guild_locale": "en-US",
    "app_permissions": "442368",
    "id": "867794291820986368",
    "locale": "en-US",
    "member": {
        "avatar": null,
        "deaf": false,
        "is_pending": false,
        "joined_at": "2020-11-02T20:46:57.364000+00:00",
        "mute": false,
        "nick": null,
        "pending": false,
        "permissions": "274877906943",
        "premium_since": null,
        "roles": ["785609923542777878"],
        "user": {
            "avatar": "a_f03401914fb4f3caa9037578ab980920",
            "discriminator": "6538",
            "id": "167348773423415296",
            "public_flags": 1,
            "username": "ian"
        }
    },
    "token": "UNIQUE_TOKEN",
    "type": 2,
    "version": 1
}"#;
        serde_json::from_str::<Interaction>(USER_COMMAND_INTERACTION).unwrap();
    }

    #[test]
    fn button_interaction() {
        const BUTTON_INTERACTION: &str = r#"{
    "version": 1,
    "type": 3,
    "token": "unique_interaction_token",
    "message": {
        "type": 0,
        "tts": false,
        "timestamp": "2021-05-19T02:12:51.710000+00:00",
        "pinned": false,
        "mentions": [],
        "mention_roles": [],
        "mention_everyone": false,
        "id": "844397162624450620",
        "flags": 0,
        "embeds": [],
        "edited_timestamp": null,
        "content": "This is a message with components.",
        "components": [
            {
                "type": 1,
                "components": [
                    {
                        "type": 2,
                        "label": "Click me!",
                        "style": 1,
                        "custom_id": "click_one"
                    }
                ]
            }
        ],
        "channel_id": "345626669114982402",
        "author": {
            "username": "Mason",
            "public_flags": 131141,
            "id": "53908232506183680",
            "discriminator": "1337",
            "avatar": "a_d5efa99b3eeaa7dd43acca82f5692432"
        },
        "attachments": []
    },
    "member": {
        "user": {
            "username": "Mason",
            "public_flags": 131141,
            "id": "53908232506183680",
            "discriminator": "1337",
            "avatar": "a_d5efa99b3eeaa7dd43acca82f5692432"
        },
        "roles": [
            "290926798626357999"
        ],
        "premium_since": null,
        "permissions": "17179869183",
        "pending": false,
        "nick": null,
        "mute": false,
        "joined_at": "2017-03-13T19:19:14.040000+00:00",
        "is_pending": false,
        "deaf": false,
        "avatar": null
    },
    "id": "846462639134605312",
    "guild_id": "290926798626357999",
    "data": {
        "custom_id": "click_one",
        "component_type": 2
    },
    "channel_id": "345626669114982999",
    "application_id": "290926444748734465"
}"#;
        serde_json::from_str::<Interaction>(BUTTON_INTERACTION).unwrap();
    }

    #[test]
    fn menu_interaction() {
        const MENU_INTERACTION: &str = r#"{
    "application_id": "845027738276462632",
    "channel_id": "772908445358620702",
    "data": {
        "component_type":3,
        "custom_id": "class_select_1",
        "values": [
            "mage",
            "rogue"
        ]
    },
    "guild_id": "772904309264089089",
    "id": "847587388497854464",
    "member": {
        "avatar": null,
        "deaf": false,
        "is_pending": false,
        "joined_at": "2020-11-02T19:25:47.248000+00:00",
        "mute": false,
        "nick": "Bot Man",
        "pending": false,
        "permissions": "17179869183",
        "premium_since": null,
        "roles": [
            "785609923542777878"
        ],
        "user":{
            "avatar": "a_d5efa99b3eeaa7dd43acca82f5692432",
            "discriminator": "1337",
            "id": "53908232506183680",
            "public_flags": 131141,
            "username": "Mason"
        }
    },
    "message":{
        "application_id": "845027738276462632",
        "attachments": [],
        "author": {
            "avatar": null,
            "bot": true,
            "discriminator": "5284",
            "id": "845027738276462632",
            "public_flags": 0,
            "username": "Interactions Test"
        },
        "channel_id": "772908445358620702",
        "components": [
            {
                "components": [
                    {
                        "custom_id": "class_select_1",
                        "max_values": 1,
                        "min_values": 1,
                        "options": [
                            {
                                "description": "Sneak n stab",
                                "emoji":{
                                    "id": "625891304148303894",
                                    "name": "rogue"
                                },
                                "label": "Rogue",
                                "value": "rogue"
                            },
                            {
                                "description": "Turn 'em into a sheep",
                                "emoji":{
                                    "id": "625891304081063986",
                                    "name": "mage"
                                },
                                "label": "Mage",
                                "value": "mage"
                            },
                            {
                                "description": "You get heals when I'm done doing damage",
                                "emoji":{
                                    "id": "625891303795982337",
                                    "name": "priest"
                                },
                                "label": "Priest",
                                "value": "priest"
                            }
                        ],
                        "placeholder": "Choose a class",
                        "type": 3
                    }
                ],
                "type": 1
            }
        ],
        "content": "Mason is looking for new arena partners. What classes do you play?",
        "edited_timestamp": null,
        "embeds": [],
        "flags": 0,
        "id": "847587334500646933",
        "interaction": {
            "id": "847587333942935632",
            "name": "dropdown",
            "type": 2,
            "user": {
                "avatar": "a_d5efa99b3eeaa7dd43acca82f5692432",
                "discriminator": "1337",
                "id": "53908232506183680",
                "public_flags": 131141,
                "username": "Mason"
            }
        },
        "mention_everyone": false,
        "mention_roles":[],
        "mentions":[],
        "pinned": false,
        "timestamp": "2021-05-27T21:29:27.956000+00:00",
        "tts": false,
        "type": 20,
        "webhook_id": "845027738276462632"
    },
    "token": "UNIQUE_TOKEN",
    "type": 3,
    "version": 1
}"#;
        serde_json::from_str::<Interaction>(MENU_INTERACTION).unwrap();
    }

    #[test]
    fn command_group() {
        const JSON: &'static str = r#"{
      "type": 1,
      "options": [
        {
          "type": 2,
          "options": [
            {
              "type": 1,
              "options": [
                {
                  "value": "1148347212850528367",
                  "type": 7,
                  "name": "channel"
                },
                {
                  "value": "<@243418816510558208>",
                  "type": 3,
                  "name": "message"
                }
              ],
              "name": "message"
            }
          ],
          "name": "post"
        }
      ],
      "name": "ll",
      "id": "1149478264100884480",
      "guild_id": "492122906864779274"
    }"#;
        serde_json::from_str::<ApplicationCommandData>(JSON).unwrap();
    }
}
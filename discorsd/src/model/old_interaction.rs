use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::convert::TryFrom;
use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};

use chrono::{DateTime, Utc};
use itertools::Itertools;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::ser::SerializeSeq;

use crate::cache::IdMap;
use crate::http::channel::{embed, MessageAttachment, RichEmbed};
use crate::model::channel::ChannelType;
use crate::model::components::{ActionRow, ComponentId};
use crate::model::guild::GuildMember;
use crate::model::ids::{ApplicationId, ChannelId, CommandId, GuildId, InteractionId, MentionableId, MessageId, RoleId, UserId};
use crate::model::locales::Locale;
use crate::model::message::{AllowedMentions, Attachment, MessageFlags};
use crate::model::new_command;
use crate::model::permissions::{Permissions, Role};
use crate::model::user::User;
use crate::serde_utils::{self, BoolExt, null_as_t};

mod validate {
    use once_cell::sync::Lazy;
    use regex::Regex;

    static NAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[-_\p{L}\p{N}\p{sc=Deva}\p{sc=Thai}]{1,32}$").unwrap());

    pub fn name(name: &str) {
        assert!(
            NAME_REGEX.is_match(name),
            "names must only contain letters, numbers, `-`, and `_` and must be 1-32 characters long; name = `{:?}`", name
        );
        assert!(
            name.chars()
                .all(|c| !c.is_uppercase()),
            "all characters in names must be lowercase if it has a lowercase variant; name = {:?}", name
        );
    }

    pub fn description(description: &str) {
        let dlen = description.chars().count();
        assert!(
            (1..=100).contains(&dlen),
            "command descriptions must be 1-100 characters long ({:?} is {} characters)",
            description, dlen
        );
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct Command {
    pub name: &'static str,
    #[serde(deserialize_with = "null_as_t", skip_serializing_if = "HashMap::is_empty")]
    pub name_localizations: HashMap<Locale, &'static str>,
    pub description: Cow<'static, str>,
    #[serde(deserialize_with = "null_as_t", skip_serializing_if = "HashMap::is_empty")]
    pub description_localizations: HashMap<Locale, String>,
    pub options: Vec<new_command::CommandOption>,
    #[serde(skip_serializing_if = "BoolExt::is_true")]
    pub default_permission: bool,
    #[serde(rename = "type", skip_serializing)]
    pub kind: ApplicationCommandType,
}

impl Command {
    pub fn chat_input<D: Into<Cow<'static, str>>>(
        name: &'static str,
        description: D,
        options: Vec<new_command::CommandOption>,
        default_permission: bool,
    ) -> Self {
        let description = description.into();
        validate::name(name);
        validate::description(&description);
        // todo
        // options.validate();
        assert!(
            name.len() + description.len()
                // todo
                // + options.text_len()
                <= 4000,
            "Maximum of 4000 bytes for combined name, description, and value properties for \
            each command and its subcommands and groups"
        );
        Self {
            name,
            name_localizations: Default::default(),
            description,
            description_localizations: Default::default(),
            options,
            default_permission,
            kind: ApplicationCommandType::ChatInput,
        }
    }
}

impl Hash for Command {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state)
    }
}

impl PartialEq for Command {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

#[derive(Debug, Clone)]
pub enum TopLevelOption {
    Groups(Vec<SubCommandGroup>),
    Commands(Vec<SubCommand>),
    Data(Vec<DataOption>),
    Empty,
}

impl TopLevelOption {
    fn text_len(&self) -> usize {
        fn group_len(group: &SubCommandGroup) -> usize {
            group.name.len()
                + group.description.len()
                + group.sub_commands.iter().map(command_len).sum::<usize>()
        }
        fn command_len(command: &SubCommand) -> usize {
            command.name.len()
                + command.description.len()
                + options_len(&command.options)
        }
        fn options_len(options: &[DataOption]) -> usize {
            options.iter()
                .map(|o| o.name().len() + o.description().len())
                .sum()
        }
        match self {
            Self::Groups(groups) => groups.iter().map(group_len).sum(),
            Self::Commands(commands) => commands.iter().map(command_len).sum(),
            Self::Data(options) => options_len(options),
            Self::Empty => 0,
        }
    }

    fn validate(&self) {
        match self {
            Self::Groups(groups) => groups.iter().for_each(Self::validate_group),
            Self::Commands(commands) => commands.iter().for_each(Self::validate_command),
            Self::Data(options) => Self::validate_options(options),
            Self::Empty => {}
        }
    }

    fn validate_group(SubCommandGroup { name, description, sub_commands }: &SubCommandGroup) {
        validate::name(name);
        validate::description(description);
        sub_commands.iter().for_each(Self::validate_command)
    }

    fn validate_command(SubCommand { name, description, options }: &SubCommand) {
        validate::name(name);
        validate::description(description);
        assert!(
            options.len() <= 25,
            "commands can have at most 25 options"
        );
        Self::validate_options(options)
    }

    fn validate_options(options: &[DataOption]) {
        assert!(
            !options.iter()
                .skip_while(|o| o.required())
                .any(DataOption::required),
            "all required options must be at front of list"
        );
        assert_eq!(
            options.iter()
                .map(DataOption::name)
                .unique()
                .count(),
            options.len(),
            "must not repeat option names"
        );
        for option in options {
            assert!(
                option.num_choices() <= 25,
                "options can have at most 25 choices"
            );
        }
    }
}

impl Serialize for TopLevelOption {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Groups(groups) => groups.serialize(s),
            Self::Commands(subs) => subs.serialize(s),
            Self::Data(opts) => opts.serialize(s),
            Self::Empty => s.serialize_seq(Some(0))?.end(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SubCommand {
    /// 1-32 character name
    pub name: Cow<'static, str>,
    /// 1-100 character description
    pub description: Cow<'static, str>,
    /// the parameters to this subcommand
    pub options: Vec<DataOption>,
}

impl Serialize for SubCommand {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        SerializeOption {
            kind: ApplicationCommandOptionType::SubCommand,
            name: self.name.clone(),
            description: self.description.clone(),
            required: false,
            choices: vec![],
            options: Some(&self.options),
        }.serialize(s)
    }
}

#[derive(Debug, Clone)]
pub struct SubCommandGroup {
    /// 1-32 character name
    pub name: Cow<'static, str>,
    /// 1-100 character description
    pub description: Cow<'static, str>,
    /// the subcommands in this subcommand group
    pub sub_commands: Vec<SubCommand>,
}

impl Serialize for SubCommandGroup {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        SerializeOption {
            kind: ApplicationCommandOptionType::SubCommandGroup,
            name: self.name.clone(),
            description: self.description.clone(),
            required: false,
            choices: vec![],
            options: Some(&self.sub_commands),
        }.serialize(s)
    }
}

#[derive(Debug, Clone)]
pub enum DataOption {
    String(CommandDataOption<&'static str>),
    Integer(CommandDataOption<i64>),
    Boolean(CommandDataOption<bool>),
    User(CommandDataOption<UserId>),
    Channel(CommandDataOption<ChannelId>),
    Role(CommandDataOption<RoleId>),
    Mentionable(CommandDataOption<MentionableId>),
    Number(CommandDataOption<f64>),
    Attachment(CommandDataOption<Attachment>),
}

impl Serialize for DataOption {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use ApplicationCommandOptionType::*;
        match self {
            Self::String(opt) => opt.serializable(String),
            Self::Integer(opt) => opt.serializable(Integer),
            Self::Boolean(opt) => opt.serializable(Boolean),
            Self::User(opt) => opt.serializable(User),
            Self::Channel(opt) => opt.serializable(Channel),
            Self::Role(opt) => opt.serializable(Role),
            Self::Mentionable(opt) => opt.serializable(Mentionable),
            Self::Number(opt) => opt.serializable(Number),
            Self::Attachment(opt) => opt.serializable(Attachment),
        }.serialize(s)
    }
}

impl DataOption {
    pub fn name(&self) -> &str {
        match self {
            Self::String(o) => o.name.as_ref(),
            Self::Integer(o) => o.name.as_ref(),
            Self::Boolean(o) => o.name.as_ref(),
            Self::User(o) => o.name.as_ref(),
            Self::Channel(o) => o.name.as_ref(),
            Self::Role(o) => o.name.as_ref(),
            Self::Mentionable(o) => o.name.as_ref(),
            Self::Number(o) => o.name.as_ref(),
            Self::Attachment(o) => o.name.as_ref(),
        }
    }
    pub fn description(&self) -> &str {
        match self {
            Self::String(o) => o.description.as_ref(),
            Self::Integer(o) => o.description.as_ref(),
            Self::Boolean(o) => o.description.as_ref(),
            Self::User(o) => o.description.as_ref(),
            Self::Channel(o) => o.description.as_ref(),
            Self::Role(o) => o.description.as_ref(),
            Self::Mentionable(o) => o.description.as_ref(),
            Self::Number(o) => o.description.as_ref(),
            Self::Attachment(o) => o.description.as_ref(),
        }
    }
    pub fn required(&self) -> bool {
        match self {
            Self::String(o) => o.required,
            Self::Integer(o) => o.required,
            Self::Boolean(o) => o.required,
            Self::User(o) => o.required,
            Self::Channel(o) => o.required,
            Self::Role(o) => o.required,
            Self::Mentionable(o) => o.required,
            Self::Number(o) => o.required,
            Self::Attachment(o) => o.required,
        }
    }
    pub(crate) fn num_choices(&self) -> usize {
        match self {
            Self::String(cdo) => cdo.choices.len(),
            Self::Integer(cdo) => cdo.choices.len(),
            Self::Number(cdo) => cdo.choices.len(),
            Self::Boolean(_)
            | Self::User(_)
            | Self::Channel(_)
            | Self::Role(_)
            | Self::Mentionable(_)
            | Self::Attachment(_) => 0,
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommandDataOption<T> {
    /// 1-32 character name
    name: Cow<'static, str>,
    /// Localization dictionary for the name field. Values follow the same restrictions as name
    name_localizations: HashMap<Locale, Cow<'static, str>>,
    /// 1-100 character description
    description: Cow<'static, str>,
    /// Localization dictionary for the description field. Values follow the same restrictions as description
    description_localizations: HashMap<Locale, Cow<'static, str>>,
    /// if the parameter is required or optional--default false
    required: bool,
    /// choices for string and int types for the user to pick from
    choices: Vec<CommandChoice<T>>,
    // todo channel types?
    /// If the option is an INTEGER or NUMBER type, the minimum value permitted
    min_value: Option<T>,
    /// If the option is an INTEGER or NUMBER type, the maximum value permitted
    max_value: Option<T>,
    /// For option type STRING, the minimum allowed length (minimum of 0, maximum of 6000)
    min_length: Option<usize>,
    /// For option type STRING, the maximum allowed length (minimum of 1, maximum of 6000)
    max_length: Option<usize>,
    /// If autocomplete interactions are enabled for this STRING, INTEGER, or NUMBER type option
    autocomplete: bool,
}

impl<T> CommandDataOption<T> {
    pub fn new<N: Into<Cow<'static, str>>, D: Into<Cow<'static, str>>>(name: N, description: D) -> Self {
        let name = name.into();
        let description = description.into();
        validate::name(&name);
        validate::description(&description);

        Self {
            name,
            name_localizations: Default::default(),
            description,
            description_localizations: Default::default(),
            required: false,
            choices: [].into(),
            min_value: None,
            max_value: None,
            min_length: None,
            max_length: None,
            autocomplete: false,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn autocomplete(mut self) -> Self {
        self.autocomplete = true;
        // todo is this how this should be handled
        assert!(self.choices.is_empty(), "Can't set choices and autocomplete");
        self
    }
}

impl CommandDataOption<&'static str> {
    pub fn new_str<N: Into<Cow<'static, str>>, D: Into<Cow<'static, str>>>(name: N, description: D) -> Self {
        Self::new(name, description)
    }

    pub fn choices(mut self, choices: Vec<CommandChoice<&'static str>>) -> Self {
        self.choices = choices;
        self
    }

    pub fn min_length(mut self, length: usize) -> Self {
        self.min_length = Some(length);
        self
    }

    pub fn max_length(mut self, length: usize) -> Self {
        self.max_length = Some(length);
        self
    }
}

impl CommandDataOption<i64> {
    pub fn new_int<N: Into<Cow<'static, str>>, D: Into<Cow<'static, str>>>(name: N, description: D) -> Self {
        Self::new(name, description)
    }

    pub fn choices(mut self, choices: Vec<CommandChoice<i64>>) -> Self {
        self.choices = choices;
        self
    }

    pub fn min_value(mut self, min: i64) -> Self {
        self.min_value = Some(min);
        self
    }

    pub fn max_value(mut self, max: i64) -> Self {
        self.max_value = Some(max);
        self
    }
}

impl CommandDataOption<f64> {
    pub fn new_num<N: Into<Cow<'static, str>>, D: Into<Cow<'static, str>>>(name: N, description: D) -> Self {
        Self::new(name, description)
    }

    pub fn choices(mut self, choices: Vec<CommandChoice<f64>>) -> Self {
        self.choices = choices;
        self
    }

    pub fn min_value(mut self, min: f64) -> Self {
        self.min_value = Some(min);
        self
    }

    pub fn max_value(mut self, max: f64) -> Self {
        self.max_value = Some(max);
        self
    }
}

impl<T> CommandDataOption<T>
    where ApplicationCommandOptionChoice: From<CommandChoice<T>>,
          CommandChoice<T>: Clone,
{
    fn serializable(&self, kind: ApplicationCommandOptionType) -> SerializeOption<DataOption> {
        // have to convert `CommandChoice<T>` to `ApplicationCommandOptionChoice` to get rid of the
        // generic type. todo is there a better way to do this? (could make choices: Option<String>?)
        let choices = self.choices
            .iter()
            // todo this used to copy them, figure out how to make it efficient again?
            .cloned()
            .map(ApplicationCommandOptionChoice::from)
            .collect();
        SerializeOption {
            kind,
            name: Cow::clone(&self.name),
            description: Cow::clone(&self.description),
            required: self.required,
            choices,
            options: None,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct CommandChoice<T> {
    /// 1-100 character choice name
    pub name: &'static str,
    /// Localization dictionary for the name field. Values follow the same restrictions as name
    pub name_localizations: HashMap<Locale, &'static str>,
    /// value of the choice
    pub value: T,
    #[serde(skip)]
    _priv: (),
}

impl<T> CommandChoice<T> {
    pub fn new(name: &'static str, value: T) -> Self {
        let nlen = name.chars().count();
        assert!(
            (1..=100).contains(&nlen),
            "command names must be 1-100 characters, name = {:?}",
            name
        );

        Self { name, name_localizations: Default::default(), value, _priv: () }
    }
}

// to help with type inference
impl CommandChoice<&'static str> {
    pub fn new_str(name_value: &'static str) -> Self {
        Self::new(name_value, name_value)
    }
}

impl<'a> From<CommandChoice<&'a str>> for ApplicationCommandOptionChoice {
    fn from(choice: CommandChoice<&'a str>) -> Self {
        Self { name: choice.name.to_string(), value: OptionValue::String(choice.value.to_string()) }
    }
}

impl From<CommandChoice<i64>> for ApplicationCommandOptionChoice {
    fn from(choice: CommandChoice<i64>) -> Self {
        Self { name: choice.name.to_string(), value: OptionValue::Integer(choice.value) }
    }
}

impl From<CommandChoice<bool>> for ApplicationCommandOptionChoice {
    fn from(choice: CommandChoice<bool>) -> Self {
        Self { name: choice.name.to_string(), value: OptionValue::Bool(choice.value) }
    }
}

impl From<CommandChoice<UserId>> for ApplicationCommandOptionChoice {
    fn from(choice: CommandChoice<UserId>) -> Self {
        let name = choice.name.to_string();
        Self { value: OptionValue::String(name.clone()), name }
    }
}

impl From<CommandChoice<ChannelId>> for ApplicationCommandOptionChoice {
    fn from(choice: CommandChoice<ChannelId>) -> Self {
        let name = choice.name.to_string();
        Self { value: OptionValue::String(name.clone()), name }
    }
}

impl From<CommandChoice<RoleId>> for ApplicationCommandOptionChoice {
    fn from(choice: CommandChoice<RoleId>) -> Self {
        let name = choice.name.to_string();
        Self { value: OptionValue::String(name.clone()), name }
    }
}

impl From<CommandChoice<MentionableId>> for ApplicationCommandOptionChoice {
    fn from(choice: CommandChoice<MentionableId>) -> Self {
        let name = choice.name.to_string();
        Self { value: OptionValue::String(name.clone()), name }
    }
}

impl From<CommandChoice<f64>> for ApplicationCommandOptionChoice {
    fn from(choice: CommandChoice<f64>) -> Self {
        Self { value: OptionValue::Number(choice.value), name: choice.name.to_string() }
    }
}

impl From<CommandChoice<Attachment>> for ApplicationCommandOptionChoice {
    fn from(_choice: CommandChoice<Attachment>) -> Self {
        // Self { value: OptionValue::Number(choice.value), name: choice.name.to_string() }
        todo!()
    }
}


#[derive(Serialize)]
struct SerializeOption<'a, O: Debug> {
    #[serde(rename = "type")]
    pub kind: ApplicationCommandOptionType,
    pub name: Cow<'static, str>,
    pub description: Cow<'static, str>,
    #[serde(skip_serializing_if = "crate::serde_utils::BoolExt::is_false")]
    pub required: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<ApplicationCommandOptionChoice>,
    #[serde(skip_serializing_if = "skip_options")]
    pub options: Option<&'a Vec<O>>,
}

#[allow(clippy::trivially_copy_pass_by_ref, clippy::ref_option_ref)]
fn skip_options<O: Debug>(options: &Option<&Vec<O>>) -> bool {
    options.filter(|vec| !vec.is_empty()).is_none()
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     fn assert_same_json_value(correct: impl AsRef<str>, modeled: impl Serialize) {
//         use serde_json::Value;
//
//         let correct: Value = serde_json::from_str(correct.as_ref()).unwrap();
//         let modeled = serde_json::to_string_pretty(&modeled).unwrap();
//         // println!("modeled = {}", modeled);
//         let modeled: Value = serde_json::from_str(&modeled).unwrap();
//
//         assert_eq!(correct, modeled);
//     }
//
//     #[test]
//     fn part1() {
//         const CORRECT1: &'static str = r#"{
//     "name": "permissions",
//     "description": "Get or edit permissions for a user or a role",
//     "options": []
// }"#;
//         let command = Command::chat_input(
//             "permissions",
//             "Get or edit permissions for a user or a role",
//             TopLevelOption::Empty,
//             true,
//         );
//
//         assert_same_json_value(CORRECT1, command);
//     }
//
//     #[test]
//     fn part2() {
//         const CORRECT2: &'static str = r#"{
//     "name": "permissions",
//     "description": "Get or edit permissions for a user or a role",
//     "options": [
//         {
//             "name": "user",
//             "description": "Get or edit permissions for a user",
//             "type": 2
//         },
//         {
//             "name": "role",
//             "description": "Get or edit permissions for a role",
//             "type": 2
//         }
//     ]
// }"#;
//         let command = Command::chat_input(
//             "permissions",
//             "Get or edit permissions for a user or a role",
//             TopLevelOption::Groups(vec![
//                 SubCommandGroup {
//                     name: "user".into(),
//                     description: "Get or edit permissions for a user".into(),
//                     sub_commands: vec![],
//                 },
//                 SubCommandGroup {
//                     name: "role".into(),
//                     description: "Get or edit permissions for a role".into(),
//                     sub_commands: vec![],
//                 },
//             ]),
//             true,
//         );
//
//         assert_same_json_value(CORRECT2, command);
//     }
//
//     #[test]
//     fn part3() {
//         const CORRECT: &'static str = r#"{
//     "name": "permissions",
//     "description": "Get or edit permissions for a user or a role",
//     "options": [
//         {
//             "name": "user",
//             "description": "Get or edit permissions for a user",
//             "type": 2,
//             "options": [
//                 {
//                     "name": "get",
//                     "description": "Get permissions for a user",
//                     "type": 1
//                 },
//                 {
//                     "name": "edit",
//                     "description": "Edit permissions for a user",
//                     "type": 1
//                 }
//             ]
//         },
//         {
//             "name": "role",
//             "description": "Get or edit permissions for a role",
//             "type": 2,
//             "options": [
//                 {
//                     "name": "get",
//                     "description": "Get permissions for a role",
//                     "type": 1
//                 },
//                 {
//                     "name": "edit",
//                     "description": "Edit permissions for a role",
//                     "type": 1
//                 }
//             ]
//         }
//     ]
// }"#;
//
//         let command = Command::chat_input(
//             "permissions",
//             "Get or edit permissions for a user or a role",
//             TopLevelOption::Groups(vec![
//                 SubCommandGroup {
//                     name: "user".into(),
//                     description: "Get or edit permissions for a user".into(),
//                     sub_commands: vec![
//                         SubCommand {
//                             name: "get".into(),
//                             description: "Get permissions for a user".into(),
//                             options: vec![],
//                         },
//                         SubCommand {
//                             name: "edit".into(),
//                             description: "Edit permissions for a user".into(),
//                             options: vec![],
//                         },
//                     ],
//                 },
//                 SubCommandGroup {
//                     name: "role".into(),
//                     description: "Get or edit permissions for a role".into(),
//                     sub_commands: vec![
//                         SubCommand {
//                             name: "get".into(),
//                             description: "Get permissions for a role".into(),
//                             options: vec![],
//                         },
//                         SubCommand {
//                             name: "edit".into(),
//                             description: "Edit permissions for a role".into(),
//                             options: vec![],
//                         },
//                     ],
//                 },
//             ]),
//             true,
//         );
//
//         assert_same_json_value(CORRECT, command)
//     }
//
//     #[test]
//     fn part4() {
//         const CORRECT: &'static str = r#"{
//     "name": "permissions",
//     "description": "Get or edit permissions for a user or a role",
//     "options": [
//         {
//             "name": "user",
//             "description": "Get or edit permissions for a user",
//             "type": 2,
//             "options": [
//                 {
//                     "name": "get",
//                     "description": "Get permissions for a user",
//                     "type": 1,
//                     "options": [
//                         {
//                             "name": "user",
//                             "description": "The user to get",
//                             "type": 6,
//                             "required": true
//                         },
//                         {
//                             "name": "channel",
//                             "description": "The channel permissions to get. If omitted, the guild permissions will be returned",
//                             "type": 7
//                         }
//                     ]
//                 },
//                 {
//                     "name": "edit",
//                     "description": "Edit permissions for a user",
//                     "type": 1,
//                     "options": [
//                         {
//                             "name": "user",
//                             "description": "The user to edit",
//                             "type": 6,
//                             "required": true
//                         },
//                         {
//                             "name": "channel",
//                             "description": "The channel permissions to edit. If omitted, the guild permissions will be edited",
//                             "type": 7
//                         }
//                     ]
//                 }
//             ]
//         },
//         {
//             "name": "role",
//             "description": "Get or edit permissions for a role",
//             "type": 2,
//             "options": [
//                 {
//                     "name": "get",
//                     "description": "Get permissions for a role",
//                     "type": 1,
//                     "options": [
//                         {
//                             "name": "role",
//                             "description": "The role to get",
//                             "type": 8,
//                             "required": true
//                         },
//                         {
//                             "name": "channel",
//                             "description": "The channel permissions to get. If omitted, the guild permissions will be returned",
//                             "type": 7
//                         }
//                     ]
//                 },
//                 {
//                     "name": "edit",
//                     "description": "Edit permissions for a role",
//                     "type": 1,
//                     "options": [
//                         {
//                             "name": "role",
//                             "description": "The role to edit",
//                             "type": 8,
//                             "required": true
//                         },
//                         {
//                             "name": "channel",
//                             "description": "The channel permissions to edit. If omitted, the guild permissions will be edited",
//                             "type": 7
//                         }
//                     ]
//                 }
//             ]
//         }
//     ]
// }"#;
//
//         let command = Command::chat_input(
//             "permissions",
//             "Get or edit permissions for a user or a role",
//             TopLevelOption::Groups(vec![
//                 SubCommandGroup {
//                     name: "user".into(),
//                     description: "Get or edit permissions for a user".into(),
//                     sub_commands: vec![
//                         SubCommand {
//                             name: "get".into(),
//                             description: "Get permissions for a user".into(),
//                             options: vec![
//                                 DataOption::User(CommandDataOption::new(
//                                     "user",
//                                     "The user to get",
//                                 ).required()),
//                                 DataOption::Channel(CommandDataOption::new(
//                                     "channel",
//                                     "The channel permissions to get. If omitted, the guild permissions will be returned",
//                                 )),
//                             ],
//                         },
//                         SubCommand {
//                             name: "edit".into(),
//                             description: "Edit permissions for a user".into(),
//                             options: vec![
//                                 DataOption::User(CommandDataOption::new(
//                                     "user",
//                                     "The user to edit",
//                                 ).required()),
//                                 DataOption::Channel(CommandDataOption::new(
//                                     "channel",
//                                     "The channel permissions to edit. If omitted, the guild permissions will be edited",
//                                 )),
//                             ],
//                         },
//                     ],
//                 },
//                 SubCommandGroup {
//                     name: "role".into(),
//                     description: "Get or edit permissions for a role".into(),
//                     sub_commands: vec![
//                         SubCommand {
//                             name: "get".into(),
//                             description: "Get permissions for a role".into(),
//                             options: vec![
//                                 DataOption::Role(CommandDataOption::new(
//                                     "role",
//                                     "The role to get",
//                                 ).required()),
//                                 DataOption::Channel(CommandDataOption::new(
//                                     "channel",
//                                     "The channel permissions to get. If omitted, the guild permissions will be returned",
//                                 )),
//                             ],
//                         },
//                         SubCommand {
//                             name: "edit".into(),
//                             description: "Edit permissions for a role".into(),
//                             options: vec![
//                                 DataOption::Role(CommandDataOption::new(
//                                     "role",
//                                     "The role to edit",
//                                 ).required()),
//                                 DataOption::Channel(CommandDataOption::new(
//                                     "channel",
//                                     "The channel permissions to edit. If omitted, the guild permissions will be edited",
//                                 )),
//                             ],
//                         },
//                     ],
//                 },
//             ]),
//             true,
//         );
//
//         if let TopLevelOption::Groups(groups) = &command.options {
//             groups.iter()
//                 .flat_map(|g| &g.sub_commands)
//                 .flat_map(|c| &c.options)
//                 .for_each(|opt| {
//                     match opt {
//                         DataOption::String(opt) => assert!(matches!(&opt.name, Cow::Borrowed(_))),
//                         DataOption::Integer(opt) => assert!(matches!(&opt.name, Cow::Borrowed(_))),
//                         DataOption::Boolean(opt) => assert!(matches!(&opt.name, Cow::Borrowed(_))),
//                         DataOption::User(opt) => assert!(matches!(&opt.name, Cow::Borrowed(_))),
//                         DataOption::Channel(opt) => assert!(matches!(&opt.name, Cow::Borrowed(_))),
//                         DataOption::Role(opt) => assert!(matches!(&opt.name, Cow::Borrowed(_))),
//                         DataOption::Mentionable(opt) => assert!(matches!(&opt.name, Cow::Borrowed(_))),
//                         DataOption::Number(opt) => assert!(matches!(&opt.name, Cow::Borrowed(_))),
//                         DataOption::Attachment(opt) => assert!(matches!(&opt.name, Cow::Borrowed(_))),
//                     }
//                 });
//         } else {
//             panic!()
//         }
//
//         assert_same_json_value(CORRECT, command);
//     }
// }

// ^ noice model ^
// lol this is actually the 'send to discord' side
// ----------------------------------------------------
// and this is the 'get back' side
// v  raw model  v

/// An application command is the base "command" model that belongs to an application.
/// This is what you are creating when you POST a new command.
///
/// A command, or each individual subcommand, can have a maximum of 10 options
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ApplicationCommand {
    /// unique id of the command
    pub id: CommandId,
    /// the type of command, defaults `1` if not set
    #[serde(rename = "type", default)]
    pub kind: Option<ApplicationCommandType>,
    /// unique id of the parent application
    pub application_id: ApplicationId,
    /// guild id of the command, if not global
    pub guild_id: Option<GuildId>,
    /// Name of command, 1-32 characters
    pub name: String,
    /// Localization dictionary for name field. Values follow the same restrictions as name
    #[serde(deserialize_with = "null_as_t", default, skip_serializing_if = "HashMap::is_empty")]
    pub name_localization: HashMap<Locale, String>,
    /// 1-100 character description
    pub description: String,
    /// Localization dictionary for description field. Values follow the same restrictions as description
    #[serde(deserialize_with = "null_as_t", default, skip_serializing_if = "HashMap::is_empty")]
    pub description_localizations: HashMap<Locale, String>,
    // todo maybe enforce that?
    /// the parameters for the command. ONLY ALLOWED FOR CHAT COMMANDS
    #[serde(default)]
    pub options: Vec<ApplicationCommandOption>,
    /// Set of permissions represented as a bit set
    pub default_member_permissions: Option<Permissions>,
    /// Indicates whether the command is available in DMs with the app, only for globally-scoped
    /// commands. By default, commands are visible.
    #[serde(default = "bool::default_true")]
    pub dm_permission: bool,
}
id_impl!(ApplicationCommand => id: CommandId);

serde_repr! {
    pub enum ApplicationCommandType: u32 {
        /// Slash commands; a text-based command that shows up when a user types /
        ChatInput = 1,
        /// A UI-based command that shows up when you right click or tap on a user
        User = 2,
        /// A UI-based command that shows up when you right click or tap on a message
        Message = 3,
    }
}

impl Default for ApplicationCommandType {
    fn default() -> Self {
        Self::ChatInput
    }
}

/// You can specify a maximum of 10 choices per option.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ApplicationCommandOption {
    /// value of ApplicationCommandOptionType
    #[serde(rename = "type")]
    pub kind: ApplicationCommandOptionType,
    /// 1-32 character name
    pub name: String,
    /// 1-100 character description
    pub description: String,
    // /// the first required option for the user to complete--only one option can be default
    // #[serde(default, skip_serializing_if = "crate::serde_utils::BoolExt::is_false")]
    // pub default: bool,
    /// if the parameter is required or optional--default false
    #[serde(default, skip_serializing_if = "bool::is_false")]
    pub required: bool,
    /// choices for string and int types for the user to pick from
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<ApplicationCommandOptionChoice>,
    /// if the option is a subcommand or subcommand group type, this nested options will be the parameters
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<ApplicationCommandOption>,
}

// honestly this would probably be best as a generic I think?
// bruh how would that work??
serde_repr! {
    pub enum ApplicationCommandOptionType: u8 {
        SubCommand = 1,
        SubCommandGroup = 2,
        String = 3,
        /// Any integer between -2^53 and 2^53
        Integer = 4,
        Boolean = 5,
        User = 6,
        /// Includes all channel types + categories
        Channel = 7,
        Role = 8,
        /// Includes users and roles
        Mentionable = 9,
        /// Any double between -2^53 and 2^53
        Number = 10,
        /// [Attachment](super::message::Attachment)
        Attachment = 11,
    }
}

/// If you specify `choices` for an option, they are the **only** valid values for a user to pick
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ApplicationCommandOptionChoice {
    /// 1-100 character choice name
    pub name: String,
    /// value of the choice
    pub value: OptionValue,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum OptionValue {
    String(String),
    Integer(i64),
    Number(f64),
    Bool(bool),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GuildApplicationCommandPermission {
    /// the id of the command
    pub id: CommandId,
    /// the id of the application the command belongs to
    pub application_id: ApplicationId,
    /// the id of the guild
    pub guild_id: GuildId,
    /// the permissions for the command in the guild
    pub permissions: Vec<CommandPermissions>,
}

/// Partial [GuildApplicationCommandPermission](GuildApplicationCommandPermission)
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GuildCommandPermissions {
    // /// the id of the command
    // pub id: CommandId,
    /// the permissions for the command in the guild
    pub permissions: Vec<CommandPermissions>,
}

/// Referred to in Discord docs as `ApplicationCommandPermissions`
#[derive(Debug, Clone, Copy)]
pub struct CommandPermissions {
    // todo https://discord.com/developers/docs/interactions/application-commands#application-command-permissions-object-application-command-permissions-structure
    // todo https://discord.com/developers/docs/interactions/application-commands#application-command-permissions-object-application-command-permissions-constants
    /// the id of the role, user, or channel
    pub id: MentionableIdPermission,
    /// true to allow, false to disallow
    pub permission: bool,
}

impl CommandPermissions {
    pub fn allow_role(role: RoleId) -> Self {
        Self {
            id: MentionableIdPermission::Role(role),
            permission: true,
        }
    }

    pub fn disallow_role(role: RoleId) -> Self {
        Self {
            id: MentionableIdPermission::Role(role),
            permission: false,
        }
    }

    pub fn allow_user(user: UserId) -> Self {
        Self {
            id: MentionableIdPermission::User(user),
            permission: true,
        }
    }

    pub fn disallow_user(user: UserId) -> Self {
        Self {
            id: MentionableIdPermission::User(user),
            permission: false,
        }
    }
}

// todo this should be a separate type for ACP, normal mentionable is just User/Role
/// Either a `UserId` or a `RoleId`
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MentionableIdPermission {
    Role(RoleId),
    User(UserId),
    Channel(ChannelId),
}

impl From<RoleId> for MentionableIdPermission {
    fn from(role: RoleId) -> Self {
        Self::Role(role)
    }
}

impl From<UserId> for MentionableIdPermission {
    fn from(user: UserId) -> Self {
        Self::User(user)
    }
}

mod acp_impl {
    use serde::de::{Error, Unexpected};

    use super::*;

    #[derive(Deserialize, Serialize)]
    struct Shim {
        #[serde(rename = "type")]
        kind: u8,
        // the actual id type doesn't matter, just pick one :)
        id: UserId,
        permission: bool,
    }

    impl Serialize for CommandPermissions {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let Self { id, permission } = *self;
            let shim = match id {
                MentionableIdPermission::Role(role) => Shim { kind: 1, id: UserId(role.0), permission },
                MentionableIdPermission::User(id) => Shim { kind: 2, id, permission },
                MentionableIdPermission::Channel(id) => Shim { kind: 3, id: UserId(id.0), permission },
            };
            shim.serialize(s)
        }
    }

    impl<'de> Deserialize<'de> for CommandPermissions {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let Shim { kind, id, permission } = Shim::deserialize(d)?;
            match kind {
                // role
                1 => {
                    let role = RoleId(id.0);
                    Ok(Self { id: MentionableIdPermission::Role(role), permission })
                }
                // user
                2 => {
                    Ok(Self { id: MentionableIdPermission::User(id), permission })
                }
                // channel
                3 => {
                    let channel = ChannelId(id.0);
                    Ok(Self { id: MentionableIdPermission::Channel(channel), permission })
                }
                #[allow(clippy::cast_lossless)]
                bad => Err(D::Error::invalid_value(Unexpected::Unsigned(bad as _), &"1 (role), 2 (user), or 3 (channel)")),
            }
        }
    }
}

/// An interaction is the base "thing" that is sent when a user invokes a command, and is the same
/// for Slash Commands and other future interaction types.
// ooh spooky ^
// lol not really
#[derive(Deserialize, Debug, Clone)]
pub struct Interaction {
    /// id of the interaction
    pub id: InteractionId,
    /// id of the application this interaction is for
    pub application_id: ApplicationId,
    /// the type of interaction
    #[serde(rename = "type")]
    pub kind: InteractionType,
    /// the command data payload
    ///
    /// This is always present on ApplicationCommand interaction types.
    /// It is optional for future-proofing against new interaction types (according to docs, but I'm
    /// cool and can just change it to be optional then :). Also will probably just be an enum)
    pub data: InteractionData,
    #[serde(flatten)]
    /// information about where this interaction was sent, whether in a guild channel or in a dm
    pub source: InteractionSource,
    /// the channel it was sent from
    pub channel_id: ChannelId,
    /// a continuation token for responding to the interaction
    pub token: String,
    // /// read-only property, always 1
    // pub version: u8,
}

/// Information about the guild and guild member that invoked this interaction
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GuildSource {
    /// The guild the interaction was sent from
    #[serde(rename = "guild_id")]
    pub id: GuildId,
    /// Guild member data for the invoking user
    pub member: GuildMember,
}

/// Information about the user that invoked this interaction
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DmSource {
    /// The user that invoked this interaction
    pub user: User,
}

/// Information about where this interaction occurred, whether in a guild channel or in a dm
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum InteractionSource {
    /// This interaction was sent in a guild, see [GuildSource](GuildSource)
    Guild(GuildSource),
    /// This interaction was sent in a dm, see [DmSource](DmSource)
    Dm(DmSource),
}

// for Error usage
impl Display for InteractionSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for InteractionSource {}

impl InteractionSource {
    pub fn guild(self) -> Option<GuildSource> {
        match self {
            Self::Guild(gs) => Some(gs),
            Self::Dm(_) => None,
        }
    }
    pub fn user(self) -> Option<User> {
        match self {
            Self::Guild(_) => None,
            Self::Dm(DmSource { user }) => Some(user),
        }
    }
}

serde_repr! {
    pub enum InteractionType: u8 {
        Ping = 1,
        ApplicationCommand = 2,
        MessageComponent = 3,
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum InteractionData {
    ApplicationCommand(ApplicationCommandData),
    MessageComponentCommand(ComponentData),
    // todo tell user/message apart by the `type` field on InteractionData
    MessageCommand { target: MessageId },
    UserCommand { target: UserId },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ComponentData {
    pub custom_id: ComponentId,
    pub component_type: u8,
    #[serde(default)]
    pub values: Vec<String>,
}

// todo rename
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
#[serde(try_from = "ApplicationCommandInteractionData")]
pub struct ApplicationCommandData {
    pub id: CommandId,
    pub name: String,
    pub options: InteractionDataOption,
    // todo?
    // pub resolved: ResolvedData,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub enum InteractionDataOption {
    Group(GroupOption),
    Command(CommandOption),
    Values(Vec<ValueOption>),
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct GroupOption {
    pub name: String,
    pub lower: CommandOption,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct CommandOption {
    pub name: String,
    pub lower: Vec<ValueOption>,
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub struct ValueOption {
    pub name: String,
    pub lower: OptionValue,
}

impl TryFrom<ApplicationCommandInteractionData> for ApplicationCommandData {
    type Error = serde_utils::Error;

    fn try_from(value: ApplicationCommandInteractionData) -> Result<Self, Self::Error> {
        use ApplicationCommandInteractionData as ACID;
        use ApplicationCommandInteractionDataOption as ACIDO;
        use ApplicationCommandInteractionDataValue as ACIDV;

        fn new_ify(options: Vec<ACIDO>) -> Vec<ValueOption> {
            options.into_iter()
                .map(|opt| ValueOption {
                    name: opt.name,
                    lower: opt.value.value()
                        .expect("There can only ever be multiple options for the value list"),
                })
                .collect()
        }
        fn yeet_first(options: Vec<ACIDO>) -> ACIDO {
            options.into_iter()
                .exactly_one()
                .expect("Already checked for 0 or > 1 options")
        }

        let ACID {
            id,
            name: data_name,
            options,
            // mostly exists for webhook bots, so we don't process it
            resolved: _
        } = value;
        let options = if options.is_empty() {
            InteractionDataOption::Values(Vec::new())
        } else if options.len() > 1 {
            InteractionDataOption::Values(new_ify(options))
        } else {
            match yeet_first(options) {
                ACIDO {
                    name: value_name,
                    value: ACIDV::Value { value }
                } => {
                    InteractionDataOption::Values(vec![ValueOption { name: value_name, lower: value }])
                }
                ACIDO {
                    name: group_or_command_name,
                    value: ACIDV::Options { options }
                } => {
                    if options.is_empty() {
                        InteractionDataOption::Command(CommandOption { name: group_or_command_name, lower: Vec::new() })
                    } else if options.len() > 1 {
                        InteractionDataOption::Command(CommandOption { name: group_or_command_name, lower: new_ify(options) })
                    } else {
                        match yeet_first(options) {
                            ACIDO {
                                name: value_name,
                                value: ACIDV::Value { value }
                            } => {
                                InteractionDataOption::Command(CommandOption {
                                    name: group_or_command_name,
                                    lower: vec![ValueOption { name: value_name, lower: value }],
                                })
                            }
                            ACIDO {
                                name: command_name,
                                value: ACIDV::Options { options }
                            } => {
                                InteractionDataOption::Group(GroupOption {
                                    name: group_or_command_name,
                                    lower: CommandOption { name: command_name, lower: new_ify(options) },
                                })
                            }
                        }
                    }
                }
            }
        };

        Ok(Self { id, name: data_name, options })
    }
}

/// Test that the more structured [InteractionData] is correctly generated from the more raw
/// [ApplicationCommandInteractionData] directly deserialized from json from Discord.
#[cfg(test)]
mod new_data_tests {
    use std::convert::TryInto;

    use ApplicationCommandInteractionData as ACID;
    use ApplicationCommandInteractionDataOption as ACIDO;
    use ApplicationCommandInteractionDataValue as ACIDV;

    use super::*;

    #[test]
    fn rules() {
        let rules_raw = ACID {
            id: CommandId(1234),
            name: "data".to_string(),
            options: vec![
                ACIDO { name: "game".to_string(), value: ACIDV::Value { value: OptionValue::String("Avalon".to_string()) } },
                ACIDO { name: "where".to_string(), value: ACIDV::Value { value: OptionValue::String("Here".to_string()) } },
            ],
            resolved: None,
        };
        let rules_new = ApplicationCommandData {
            id: CommandId(1234),
            name: "data".to_string(),
            options: InteractionDataOption::Values(vec![
                ValueOption { name: "game".to_string(), lower: OptionValue::String("Avalon".to_string()) },
                ValueOption { name: "where".to_string(), lower: OptionValue::String("Here".to_string()) },
            ]),
        };
        assert_eq!(rules_new, rules_raw.try_into().unwrap());
    }

    #[test]
    fn perms() {
        let perms_raw = ACID {
            id: CommandId(1234),
            name: "perms".to_string(),
            options: vec![ACIDO {
                name: "user".to_string(),
                value: ACIDV::Options {
                    options: vec![ACIDO {
                        name: "edit".to_string(),
                        value: ACIDV::Options {
                            options: vec![
                                ACIDO { name: "user".to_string(), value: ACIDV::Value { value: OptionValue::String("5678".to_string()) } },
                                ACIDO { name: "channel".to_string(), value: ACIDV::Value { value: OptionValue::String("0987".to_string()) } },
                            ]
                        },
                    }]
                },
            }],
            resolved: None,
        };
        let perms_new = ApplicationCommandData {
            id: CommandId(1234),
            name: "perms".to_string(),
            options: InteractionDataOption::Group(GroupOption {
                name: "user".to_string(),
                lower: CommandOption {
                    name: "edit".to_string(),
                    lower: vec![
                        ValueOption { name: "user".to_string(), lower: OptionValue::String("5678".to_string()) },
                        ValueOption { name: "channel".to_string(), lower: OptionValue::String("0987".to_string()) },
                    ],
                },
            }),
        };
        assert_eq!(perms_new, perms_raw.try_into().unwrap())
    }

    #[test]
    fn roles_add() {
        let roles_add_raw = ACID {
            id: CommandId(1234),
            name: "roles".to_string(),
            options: vec![ACIDO {
                name: "add".to_string(),
                value: ACIDV::Options {
                    options: vec![
                        ACIDO { name: "role1".to_string(), value: ACIDV::Value { value: OptionValue::String("Assassin".to_string()) } },
                        ACIDO { name: "role2".to_string(), value: ACIDV::Value { value: OptionValue::String("Merlin".to_string()) } },
                        ACIDO { name: "role3".to_string(), value: ACIDV::Value { value: OptionValue::String("Mordred".to_string()) } },
                        ACIDO { name: "role4".to_string(), value: ACIDV::Value { value: OptionValue::String("Percival".to_string()) } },
                    ]
                },
            }],
            resolved: None,
        };
        let roles_add_new = ApplicationCommandData {
            id: CommandId(1234),
            name: "roles".to_string(),
            options: InteractionDataOption::Command(CommandOption {
                name: "add".to_string(),
                lower: vec![
                    ValueOption { name: "role1".to_string(), lower: OptionValue::String("Assassin".to_string()) },
                    ValueOption { name: "role2".to_string(), lower: OptionValue::String("Merlin".to_string()) },
                    ValueOption { name: "role3".to_string(), lower: OptionValue::String("Mordred".to_string()) },
                    ValueOption { name: "role4".to_string(), lower: OptionValue::String("Percival".to_string()) },
                ],
            }),
        };
        assert_eq!(roles_add_new, roles_add_raw.try_into().unwrap());
    }

    #[test]
    fn roles_clear() {
        let roles_clear_raw = ACID {
            id: CommandId(1234),
            name: "roles".to_string(),
            options: vec![ACIDO { name: "".to_string(), value: ACIDV::Options { options: vec![] } }],
            resolved: None,
        };
        let roles_clear_new = ApplicationCommandData {
            id: CommandId(1234),
            name: "roles".to_string(),
            options: InteractionDataOption::Command(CommandOption {
                name: "".to_string(),
                lower: vec![],
            }),
        };
        assert_eq!(roles_clear_new, roles_clear_raw.try_into().unwrap());
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ApplicationCommandInteractionData {
    /// the ID of the invoked command
    pub id: CommandId,
    /// the name of the invoked command
    pub name: String,
    /// the params + values from the user
    #[serde(default)]
    pub options: Vec<ApplicationCommandInteractionDataOption>,
    /// the values of role/user/channel parameters in the command
    pub resolved: Option<ApplicationCommandInteractionDataResolved>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ApplicationCommandInteractionDataResolved {
    pub users: Option<IdMap<User>>,
    pub members: Option<HashMap<String, PartialGuildMember>>,
    pub roles: Option<IdMap<Role>>,
    pub channels: Option<IdMap<PartialChannel>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PartialChannel {
    /// the id of this channel
    pub id: ChannelId,
    /// the name of the channel (2-100 characters)
    pub name: String,
    #[serde(rename = "type")]
    pub kind: ChannelType,
    /// not in Discord's documentation
    pub permissions: Permissions,
}
id_impl!(PartialChannel => id: ChannelId);

/// All options have names, and an option can either be a parameter and input value--in which case
/// `value` will be set--or it can denote a subcommand or group--in which case it will contain a
/// top-level key and another array of `options`.
///
/// `value` and `options` are mutually exclusive.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ApplicationCommandInteractionDataOption {
    /// the name of the parameter
    pub name: String,
    #[serde(flatten)]
    pub value: ApplicationCommandInteractionDataValue,
    // /// the value of the pair
    // pub value: Option<OptionValue>,
    // /// present if this option is a group or subcommand
    // #[serde(default)]
    // pub options: Vec<ApplicationCommandInteractionDataOption>,
}

impl ApplicationCommandInteractionDataValue {
    pub fn value(self) -> Option<OptionValue> {
        match self {
            Self::Value { value } => Some(value),
            Self::Options { .. } => None,
        }
    }
    pub fn options(self) -> Option<Vec<ApplicationCommandInteractionDataOption>> {
        match self {
            Self::Value { .. } => None,
            Self::Options { options } => Some(options),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum ApplicationCommandInteractionDataValue {
    Value {
        value: OptionValue,
    },
    Options {
        #[serde(default)]
        options: Vec<ApplicationCommandInteractionDataOption>,
    },
}

/// After receiving an interaction, you must respond to acknowledge it. This may be a `pong` for a
/// `ping`, a message, or simply an acknowledgement that you have received it and will handle the
/// command async.
///
/// Interaction responses may choose to "eat" the user's command input if you do not wish to have
/// their slash command show up as message in chat. This may be helpful for slash commands, or
/// commands whose responses are asynchronous or ephemeral messages.
#[derive(Debug, Clone)]
pub enum InteractionResponse {
    /// ACK a `Ping`
    Pong,
    /// ACK a command without sending a message, showing the user's input
    ChannelMessageWithSource(InteractionMessage),
    /// respond with a message, eating the user's input
    DeferredChannelMessageWithSource,
    /// for components ONLY, ACK an interaction and edit the original message later; the user does
    /// not see a loading state
    DeferredUpdateMessage,
    /// for components ONLY, edit the message the component was attached to
    UpdateMessage(InteractionMessage),
}

impl Serialize for InteractionResponse {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Shim<'a> {
            #[serde(rename = "type")]
            kind: u8,
            data: Option<&'a InteractionMessage>,
        }

        let shim = match self {
            Self::Pong => Shim { kind: 1, data: None },
            Self::ChannelMessageWithSource(m) => Shim { kind: 4, data: Some(m) },
            Self::DeferredChannelMessageWithSource => Shim { kind: 5, data: None },
            Self::DeferredUpdateMessage => Shim { kind: 6, data: None },
            Self::UpdateMessage(m) => Shim { kind: 7, data: Some(m) },
        };

        shim.serialize(s)
    }
}

/// Not all message fields are currently supported by Discord.
#[derive(Serialize, Debug, Clone, Default)]
pub struct InteractionMessage {
    /// is the response TTS
    pub tts: bool,
    /// message content
    pub(crate) content: Cow<'static, str>,
    /// supports up to 10 embeds
    pub(crate) embeds: Vec<RichEmbed>,
    /// allowed mentions object
    pub allowed_mentions: Option<AllowedMentions>,
    /// Only [MessageFlags::EPHEMERAL] and [MessageFlags::SUPPRESS_EMBEDS] are allowed
    flags: MessageFlags,
    /// message components
    components: Vec<ActionRow>,
    // todo? does this work
    #[serde(skip_serializing)]
    pub files: HashSet<MessageAttachment>,
}

pub fn ephemeral<C: Into<Cow<'static, str>>>(content: C) -> InteractionMessage {
    message(|m| {
        m.content(content);
        m.ephemeral();
    })
}

pub fn message<F: FnOnce(&mut InteractionMessage)>(builder: F) -> InteractionMessage {
    InteractionMessage::build(builder)
}

impl<S: Into<Cow<'static, str>>> From<S> for InteractionMessage {
    fn from(s: S) -> Self {
        message(|m| m.content(s))
    }
}

impl From<RichEmbed> for InteractionMessage {
    fn from(e: RichEmbed) -> Self {
        message(|m| m.embeds = vec![e])
    }
}

impl InteractionMessage {
    pub fn build_with<F: FnOnce(&mut Self)>(mut with: Self, builder: F) -> Self {
        builder(&mut with);
        with
    }

    pub fn build<F: FnOnce(&mut Self)>(builder: F) -> Self {
        Self::build_with(Self::default(), builder)
    }

    /// Add an embed to this [InteractionMessage](InteractionMessage).
    ///
    /// # Panics
    ///
    /// If this message already has 10 or more embeds. See also [`try_embed`](Self::try_embed).
    pub fn embed<F: FnOnce(&mut RichEmbed)>(&mut self, builder: F) {
        self.try_embed(builder)
            .map_err(|_| "can't send more than 10 embeds")
            .unwrap()
    }

    /// Add an embed to the [InteractionMessage](InteractionMessage).
    ///
    /// # Errors
    ///
    /// Returns `Err(builder)` if this message already has 10 or more embeds. See also
    /// [embed](Self::embed).
    pub fn try_embed<F: FnOnce(&mut RichEmbed)>(&mut self, builder: F) -> Result<(), F> {
        if self.embeds.len() >= 10 {
            Err(builder)
        } else {
            self.embeds.push(embed(builder));
            Ok(())
        }
    }

    pub fn content<S: Into<Cow<'static, str>>>(&mut self, content: S) {
        self.content = content.into();
    }

    pub fn ephemeral(&mut self) {
        self.flags.set(MessageFlags::EPHEMERAL, true);
    }

    // pub fn button<B, Btn>(&mut self, state: &BotState<B>, button: Btn)
    //     where B: Send + Sync + 'static,
    //           Btn: ButtonCommand<Bot=B> + 'static,
    // {
    //     self.buttons(state, [Box::new(button) as _])
    // }
    //
    // pub fn buttons<B, I>(&mut self, state: &BotState<B>, buttons: I)
    //     where B: Send + Sync + 'static,
    //           I: IntoIterator<Item=Box<dyn ButtonCommand<Bot=B>>>,
    // {
    //     let mut component_buttons = Vec::new();
    //     for button in buttons {
    //         component_buttons.push(Button::new(state, button));
    //     }
    //     self.components.push(ActionRow::buttons(component_buttons));
    // }
    //
    // pub fn menu<B, M>(&mut self, state: &BotState<B>, menu: M)
    //     where B: Send + Sync + 'static,
    //           M: MenuCommand<Bot=B> + 'static,
    // {
    //     let menu = state.make_string_menu(Box::new(menu));
    //     self.components.push(ActionRow::select_menu(menu))
    // }
}
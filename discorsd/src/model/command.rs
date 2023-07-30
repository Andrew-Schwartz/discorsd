use std::borrow::Cow;
use std::collections::HashMap;

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::commands::{CommandData, SlashCommandRaw};
use crate::model::channel::ChannelType;
use crate::model::ids::{ApplicationId, ChannelId, CommandId, MentionableId, RoleId, UserId};
use crate::model::locales::Locale;
use crate::model::message::Attachment;
use crate::model::permissions::Permissions;
use crate::serde_utils::null_as_t;

// todo
/// `CHAT_INPUT` command names and command option names must match the following regex
/// `^[-_\p{L}\p{N}\p{sc=Deva}\p{sc=Thai}]{1,32}$` with the unicode flag set. If there is a
/// lowercase variant of any letters used, you must use those. Characters with no lowercase variants
/// and/or uncased letters are still allowed. `USER` and `MESSAGE` commands may be mixed case and
/// can include spaces.
static NAME_REGEX: Lazy<Regex> = Lazy::new(||
    Regex::new(r#"^[-_\p{L}\p{N}\p{sc=Deva}\p{sc=Thai}]{1,32}$"#).unwrap()
);

#[derive(Deserialize, Serialize, Debug)]
pub struct ApplicationCommand {
    /// Unique ID of command
    pub id: CommandId,
    // /// Type of command, defaults to 1
    // pub type: one of application command type,
    /// ID of the parent application
    pub application_id: ApplicationId,
    /// Guild ID of the command, if not global
    #[serde(default)]
    pub guild_id: Option<CommandId>,
    /// Set of permissions represented as a bit set
    #[serde(default)]
    pub default_member_permissions: Option<Permissions>,
    /// Indicates whether the command is available in DMs with the app, only for globally-scoped commands. By default, commands are visible.
    #[serde(default)]
    pub dm_permission: bool,
    /// Indicates whether the command is age-restricted, defaults to false
    #[serde(default)]
    pub nsfw: bool,
    // /// Auto-incrementing version identifier updated during substantial record changes
    // pub version: usize,
    #[serde(flatten)]
    pub command: Command,
}
id_impl!(ApplicationCommand => CommandId);

serde_num_tag! {
    /// This command is sent to Discord
    #[derive(Debug, PartialEq)]
    pub enum Command = "type": u8 as CommandType {
        /// Slash commands; a text-based command that shows up when a user types /
        (1) = SlashCommand {
            /// Name of command, 1-32 characters
            name: Cow<'static, str>,
            /// Localization dictionary for name field. Values follow the same restrictions as name
            #serde = deserialize_with = "null_as_t"
            #serde = default
            #serde = skip_serializing_if = "HashMap::is_empty"
            name_localizations: HashMap<Locale, Cow<'static, str>>,
            /// Description of command, 1-100 characters
            description: Cow<'static, str>,
            /// Localization dictionary for description field. Values follow the same restrictions as description
            #serde = deserialize_with = "null_as_t"
            #serde = default
            #serde = skip_serializing_if = "HashMap::is_empty"
            description_localizations: HashMap<Locale, Cow<'static, str>>,
            /// Parameters for the command, max of 25
            #serde = default
            options: Vec<CommandOption>,
        },
        /// A UI-based command that shows up when you right click or tap on a user
        (2) = UserCommand {
            /// Name of command, 1-32 characters
            name: Cow<'static, str>,
            /// Localization dictionary for name field. Values follow the same restrictions as name
            #serde = deserialize_with = "null_as_t"
            #serde = default
            #serde = skip_serializing_if = "HashMap::is_empty"
            name_localizations: HashMap<Locale, Cow<'static, str>>,
        },
        /// A UI-based command that shows up when you right click or tap on a message
        (3) = MessageCommand {
            /// Name of command, 1-32 characters
            name: Cow<'static, str>,
            /// Localization dictionary for name field. Values follow the same restrictions as name
            #serde = deserialize_with = "null_as_t"
            #serde = default
            #serde = skip_serializing_if = "HashMap::is_empty"
            name_localizations: HashMap<Locale, Cow<'static, str>>,
        },
    }
}

impl Command {
    pub fn chat_input(
        name: &'static str,
        description: Cow<'static, str>,
        options: Vec<CommandOption>,
    ) -> Self {
        // todo validate
        Self::SlashCommand {
            name: name.into(),
            name_localizations: Default::default(),
            description: description.into(),
            description_localizations: Default::default(),
            options,
        }
    }
    pub fn user_command(
        name: &'static str,
    ) -> Self {
        // todo validate
        Self::UserCommand {
            name: name.into(),
            name_localizations: Default::default(),
        }
    }
    pub fn message_command(
        name: &'static str,
    ) -> Self {
        // todo validate
        Self::MessageCommand {
            name: name.into(),
            name_localizations: Default::default(),
        }
    }
}

serde_num_tag! {
    #[derive(Debug, Clone, PartialEq)]
    pub enum CommandOption = "type": u8 as CommandOptionType {
        (1) = SubCommand(OptionData<SubCommand>),
        (2) = SubCommandGroup(OptionData<SubCommandGroup>),
        (3) = String(OptionData<String>),
        /// Any integer between -2^53 and 2^53
        (4) = Integer(OptionData<i64>),
        (5) = Boolean(OptionData<bool>),
        (6) = User(OptionData<UserId>),
        /// Includes all channel types + categories
        (7) = Channel(OptionData<ChannelId>),
        (8) = Role(OptionData<RoleId>),
        /// Includes users and roles
        (9) = Mentionable(OptionData<MentionableId>),
        /// Any double between -2^53 and 2^53
        (10) = Number(OptionData<f64>),
        /// attachment object
        (11) = Attachment(OptionData<Attachment>)
    }
}

impl From<CommandDataOption> for CommandOption {
    fn from(value: CommandDataOption) -> Self {
        match value {
            CommandDataOption::String(d) => Self::String(d),
            CommandDataOption::Integer(d) => Self::Integer(d),
            CommandDataOption::Boolean(d) => Self::Boolean(d),
            CommandDataOption::User(d) => Self::User(d),
            CommandDataOption::Channel(d) => Self::Channel(d),
            CommandDataOption::Role(d) => Self::Role(d),
            CommandDataOption::Mentionable(d) => Self::Mentionable(d),
            CommandDataOption::Number(d) => Self::Number(d),
            CommandDataOption::Attachment(d) => Self::Attachment(d),
        }
    }
}

impl From<SubCommandOption> for CommandOption {
    fn from(SubCommandOption::SubCommand(d): SubCommandOption) -> Self {
        Self::SubCommand(d)
    }
}

impl From<SubCommandGroupOption> for CommandOption {
    fn from(SubCommandGroupOption::SubCommandGroup(d): SubCommandGroupOption) -> Self {
        Self::SubCommandGroup(d)
    }
}

serde_num_tag! {
    #[derive(Debug, Clone, PartialEq)]
    pub enum CommandDataOption = "type": CommandOptionType {
        (CommandOptionType::String) = String(OptionData<String>),
        /// Any integer between -2^53 and 2^53
        (CommandOptionType::Integer) = Integer(OptionData<i64>),
        (CommandOptionType::Boolean) = Boolean(OptionData<bool>),
        (CommandOptionType::User) = User(OptionData<UserId>),
        /// Includes all channel types + categories
        (CommandOptionType::Channel) = Channel(OptionData<ChannelId>),
        (CommandOptionType::Role) = Role(OptionData<RoleId>),
        /// Includes users and roles
        (CommandOptionType::Mentionable) = Mentionable(OptionData<MentionableId>),
        /// Any double between -2^53 and 2^53
        (CommandOptionType::Number) = Number(OptionData<f64>),
        /// attachment object
        (CommandOptionType::Attachment) = Attachment(OptionData<Attachment>)
    }
}

serde_num_tag! {
    #[derive(Debug, Clone, PartialEq)]
    pub enum SubCommandOption = "type": CommandOptionType {
        (CommandOptionType::SubCommand) = SubCommand(OptionData<SubCommand>),
    }
}

serde_num_tag! {
    #[derive(Debug, Clone, PartialEq)]
    pub enum SubCommandGroupOption = "type": CommandOptionType {
        (CommandOptionType::SubCommandGroup) = SubCommandGroup(OptionData<SubCommandGroup>),
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct OptionData<T: OptionType> {
    /// 1-32 character name
    pub name: Cow<'static, str>,
    /// Localization dictionary for name field. Values follow the same restrictions as name
    #[serde(deserialize_with = "null_as_t", default, skip_serializing_if = "HashMap::is_empty")]
    pub name_localizations: HashMap<Locale, String>,
    /// 1-100 character description
    pub description: Cow<'static, str>,
    /// Localization dictionary for description field. Values follow the same restrictions as description
    #[serde(deserialize_with = "null_as_t", default, skip_serializing_if = "HashMap::is_empty")]
    pub description_localizations: HashMap<Locale, String>,
    /// Type specific extra data
    #[serde(flatten)]
    pub extra_data: T::Data,
}

impl<T: OptionType> OptionData<T> {
    pub fn new<N, D>(name: N, desc: D) -> Self
        where N: Into<Cow<'static, str>>,
              D: Into<Cow<'static, str>>,
    {
        Self {
            name: name.into(),
            name_localizations: Default::default(),
            description: desc.into(),
            description_localizations: Default::default(),
            extra_data: T::Data::default(),
        }
    }
}

impl<T: OptionType> OptionData<T> {
    pub fn set_choices<C, SCR>(&mut self, choices: Vec<C>)
        where C: ToString + CommandData<SCR, ChoicePrimitive=T::Choice>, /* ! */
              SCR: SlashCommandRaw,
    {
        let choices = choices.into_iter()
            .map(|c| c.into_command_choice())
            .collect();
        //             &mut StringData       Vec<Choice<String>>
        T::set_choices(&mut self.extra_data, choices)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Choice<T> {
    /// 1-32 character name
    pub name: Cow<'static, str>,
    /// Localization dictionary for name field. Values follow the same restrictions as name
    #[serde(deserialize_with = "null_as_t", default, skip_serializing_if = "HashMap::is_empty")]
    pub name_localizations: HashMap<Locale, String>,
    /// Value for the choice, up to 100 characters if string
    pub value: T,
}

impl<T> Choice<T> {
    pub fn new<N: Into<Cow<'static, str>>>(name: N, value: T) -> Self {
        let name = name.into();
        let nlen = name.chars().count();
        assert!(
            (1..=100).contains(&nlen),
            "command names must be 1-100 characters, name = {:?}",
            name
        );

        Self { name, name_localizations: Default::default(), value }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
pub struct SubCommandGroup {
    /// the subcommands in this subcommand group
    // todo this can only be SubCommand
    #[serde(default, rename = "options")]
    pub sub_commands: Vec<SubCommandOption>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
pub struct SubCommand {
    /// the parameters to this subcommand
    // todo enforce that it can't be another SubCommand or SubCommandGroup
    #[serde(default, rename = "options")]
    pub data_options: Vec<CommandDataOption>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
pub struct StringData {
    /// If the parameter is required or optional--default false
    #[serde(default)]
    pub required: bool,
    /// choices to pick from, max 25
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<Choice<String>>,
    /// the minimum allowed length (minimum of 0, maximum of 6000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    /// the maximum allowed length (minimum of 1, maximum of 6000)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    /// If autocomplete interactions are enabled for this option
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autocomplete: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
pub struct NumericData<T> {
    /// If the parameter is required or optional--default false
    #[serde(default)]
    pub required: bool,
    /// choices to pick from, max 25
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<Choice<T>>,
    /// the minimum value permitted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<T>,
    /// the maximum value permitted
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<T>,
    /// If autocomplete interactions are enabled for this option
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autocomplete: Option<bool>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
pub struct ChannelData {
    /// If the parameter is required or optional--default false
    #[serde(default)]
    pub required: bool,
    /// the channels shown will be restricted to these types
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub choices: Vec<ChannelType>,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Default)]
pub struct ParameterData {
    /// If the parameter is required or optional--default false
    #[serde(default)]
    pub required: bool,
}

pub trait OptionType {
    type Data: Default;
    type Choice;

    fn set_choices(data: &mut Self::Data, choices: Vec<Choice<Self::Choice>>);
}
macro_rules! data_type {
    ($($t:ty => $data:ty, $choice:ty, $d:ident, $c:ident, $set_choices:block);+ $(;)?) => {
        $(
            impl OptionType for $t {
                type Data = $data;
                type Choice = $choice;
                fn set_choices($d: &mut Self::Data, $c: Vec<Choice<Self::Choice>>) $set_choices
            }
        )+
    };
}

// impl OptionType for String {
//     type Data = StringData;
//     type Choice = String;
//
//     fn set_choices(data: &mut Self::Data, choices: Vec<Choice<Self::Choice>>) {
//         data.choices = choices;
//     }
// }

data_type! {
    SubCommand => SubCommand, std::convert::Infallible, _d, _c, { unreachable!() };
    SubCommandGroup => SubCommandGroup, std::convert::Infallible, _d, _c, { unreachable!() };
    String => StringData, String, data, choices, { data.choices = choices };
    i64 => NumericData<i64>, i64, data, choices, { data.choices = choices };
    bool => ParameterData, std::convert::Infallible, _d, _c, {};
    UserId => ParameterData, std::convert::Infallible, _d, _c, {};
    ChannelId => ChannelData, std::convert::Infallible, _d, _c, {};
    RoleId => ParameterData, std::convert::Infallible, _d, _c, {};
    MentionableId => ParameterData, std::convert::Infallible, _d, _c, {};
    f64 => NumericData<f64>, f64, data, choices, { data.choices = choices };
    Attachment => ParameterData, std::convert::Infallible, _d, _c, {};
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test(correct: &'static str, command: Command) {
        let serialized = serde_json::to_string_pretty(&command).unwrap();
        assert_eq!(serialized, correct);
        let deserialized: Command = serde_json::from_str(correct).unwrap();
        assert_eq!(deserialized, command);
    }

    #[test]
    fn slash_command() {
        const CORRECT: &str = r#"{
  "type": 1,
  "name": "blep",
  "description": "Send a random adorable animal photo",
  "options": [
    {
      "type": 3,
      "name": "animal",
      "description": "The type of animal",
      "required": true,
      "choices": [
        {
          "name": "Dog",
          "value": "animal_dog"
        },
        {
          "name": "Cat",
          "value": "animal_cat"
        },
        {
          "name": "Penguin",
          "value": "animal_penguin"
        }
      ]
    },
    {
      "type": 5,
      "name": "only_smol",
      "description": "Whether to show only baby animals",
      "required": false
    }
  ]
}"#;
        let command = Command::SlashCommand {
            name: "blep".into(),
            name_localizations: Default::default(),
            description: "Send a random adorable animal photo".into(),
            description_localizations: Default::default(),
            options: vec![
                CommandOption::String(OptionData {
                    name: "animal".into(),
                    name_localizations: Default::default(),
                    description: "The type of animal".into(),
                    description_localizations: Default::default(),
                    extra_data: StringData {
                        required: true,
                        choices: vec![
                            Choice {
                                name: "Dog".into(),
                                name_localizations: Default::default(),
                                value: "animal_dog".into(),
                            },
                            Choice {
                                name: "Cat".into(),
                                name_localizations: Default::default(),
                                value: "animal_cat".into(),
                            },
                            Choice {
                                name: "Penguin".into(),
                                name_localizations: Default::default(),
                                value: "animal_penguin".into(),
                            },
                        ],
                        min_length: None,
                        max_length: None,
                        autocomplete: None,
                    },
                }),
                CommandOption::Boolean(OptionData {
                    name: "only_smol".into(),
                    name_localizations: Default::default(),
                    description: "Whether to show only baby animals".into(),
                    description_localizations: Default::default(),
                    extra_data: ParameterData {
                        required: false,
                    },
                }),
            ],
        };
        test(CORRECT, command)
    }

    #[test]
    fn no_options() {
        const CORRECT: &str = r#"{
  "type": 1,
  "name": "permissions",
  "description": "Get or edit permissions for a user or a role",
  "options": []
}"#;
        let command = Command::SlashCommand {
            name: "permissions".into(),
            name_localizations: Default::default(),
            description: "Get or edit permissions for a user or a role".into(),
            description_localizations: Default::default(),
            options: vec![],
        };
        test(CORRECT, command);
    }

    #[test]
    fn sub_command_groups() {
        const CORRECT: &str = r#"{
  "type": 1,
  "name": "permissions",
  "description": "Get or edit permissions for a user or a role",
  "options": [
    {
      "type": 2,
      "name": "user",
      "description": "Get or edit permissions for a user",
      "options": []
    },
    {
      "type": 2,
      "name": "role",
      "description": "Get or edit permissions for a role",
      "options": []
    }
  ]
}"#;
        let command = Command::SlashCommand {
            name: "permissions".into(),
            name_localizations: Default::default(),
            description: "Get or edit permissions for a user or a role".into(),
            description_localizations: Default::default(),
            options: vec![
                CommandOption::SubCommandGroup(OptionData {
                    name: "user".into(),
                    name_localizations: Default::default(),
                    description: "Get or edit permissions for a user".into(),
                    description_localizations: Default::default(),
                    extra_data: SubCommandGroup {
                        sub_commands: vec![],
                    },
                }),
                CommandOption::SubCommandGroup(OptionData {
                    name: "role".into(),
                    name_localizations: Default::default(),
                    description: "Get or edit permissions for a role".into(),
                    description_localizations: Default::default(),
                    extra_data: SubCommandGroup {
                        sub_commands: vec![],
                    },
                }),
            ],
        };
        test(CORRECT, command);
    }

    #[test]
    fn sub_commands() {
        const CORRECT: &str = r#"{
  "type": 1,
  "name": "permissions",
  "description": "Get or edit permissions for a user or a role",
  "options": [
    {
      "type": 2,
      "name": "user",
      "description": "Get or edit permissions for a user",
      "options": [
        {
          "type": 1,
          "name": "get",
          "description": "Get permissions for a user",
          "options": []
        },
        {
          "type": 1,
          "name": "edit",
          "description": "Edit permissions for a user",
          "options": []
        }
      ]
    },
    {
      "type": 2,
      "name": "role",
      "description": "Get or edit permissions for a role",
      "options": [
        {
          "type": 1,
          "name": "get",
          "description": "Get permissions for a role",
          "options": []
        },
        {
          "type": 1,
          "name": "edit",
          "description": "Edit permissions for a role",
          "options": []
        }
      ]
    }
  ]
}"#;
        let command = Command::SlashCommand {
            name: "permissions".into(),
            name_localizations: Default::default(),
            description: "Get or edit permissions for a user or a role".into(),
            description_localizations: Default::default(),
            options: vec![
                CommandOption::SubCommandGroup(OptionData {
                    name: "user".into(),
                    name_localizations: Default::default(),
                    description: "Get or edit permissions for a user".into(),
                    description_localizations: Default::default(),
                    extra_data: SubCommandGroup {
                        sub_commands: vec![
                            SubCommandOption::SubCommand(OptionData {
                                name: "get".into(),
                                name_localizations: Default::default(),
                                description: "Get permissions for a user".into(),
                                description_localizations: Default::default(),
                                extra_data: SubCommand {
                                    data_options: vec![],
                                },
                            }),
                            SubCommandOption::SubCommand(OptionData {
                                name: "edit".into(),
                                name_localizations: Default::default(),
                                description: "Edit permissions for a user".into(),
                                description_localizations: Default::default(),
                                extra_data: SubCommand {
                                    data_options: vec![],
                                },
                            }),
                        ],
                    },
                }),
                CommandOption::SubCommandGroup(OptionData {
                    name: "role".into(),
                    name_localizations: Default::default(),
                    description: "Get or edit permissions for a role".into(),
                    description_localizations: Default::default(),
                    extra_data: SubCommandGroup {
                        sub_commands: vec![
                            SubCommandOption::SubCommand(OptionData {
                                name: "get".into(),
                                name_localizations: Default::default(),
                                description: "Get permissions for a role".into(),
                                description_localizations: Default::default(),
                                extra_data: SubCommand {
                                    data_options: vec![],
                                },
                            }),
                            SubCommandOption::SubCommand(OptionData {
                                name: "edit".into(),
                                name_localizations: Default::default(),
                                description: "Edit permissions for a role".into(),
                                description_localizations: Default::default(),
                                extra_data: SubCommand {
                                    data_options: vec![],
                                },
                            }),
                        ],
                    },
                }),
            ],
        };
        test(CORRECT, command);
    }

    #[test]
    fn example_command() {
        const CORRECT: &str = r#"{
  "type": 1,
  "name": "permissions",
  "description": "Get or edit permissions for a user or a role",
  "options": [
    {
      "type": 2,
      "name": "user",
      "description": "Get or edit permissions for a user",
      "options": [
        {
          "type": 1,
          "name": "get",
          "description": "Get permissions for a user",
          "options": [
            {
              "type": 6,
              "name": "user",
              "description": "The user to get",
              "required": true
            },
            {
              "type": 7,
              "name": "channel",
              "description": "The channel permissions to get. If omitted, the guild permissions will be returned",
              "required": false
            }
          ]
        },
        {
          "type": 1,
          "name": "edit",
          "description": "Edit permissions for a user",
          "options": [
            {
              "type": 6,
              "name": "user",
              "description": "The user to edit",
              "required": true
            },
            {
              "type": 7,
              "name": "channel",
              "description": "The channel permissions to edit. If omitted, the guild permissions will be edited",
              "required": false
            }
          ]
        }
      ]
    },
    {
      "type": 2,
      "name": "role",
      "description": "Get or edit permissions for a role",
      "options": [
        {
          "type": 1,
          "name": "get",
          "description": "Get permissions for a role",
          "options": [
            {
              "type": 8,
              "name": "role",
              "description": "The role to get",
              "required": true
            },
            {
              "type": 7,
              "name": "channel",
              "description": "The channel permissions to get. If omitted, the guild permissions will be returned",
              "required": false
            }
          ]
        },
        {
          "type": 1,
          "name": "edit",
          "description": "Edit permissions for a role",
          "options": [
            {
              "type": 8,
              "name": "role",
              "description": "The role to edit",
              "required": true
            },
            {
              "type": 7,
              "name": "channel",
              "description": "The channel permissions to edit. If omitted, the guild permissions will be edited",
              "required": false
            }
          ]
        }
      ]
    }
  ]
}"#;
        let command = Command::SlashCommand {
            name: "permissions".into(),
            name_localizations: Default::default(),
            description: "Get or edit permissions for a user or a role".into(),
            description_localizations: Default::default(),
            options: vec![
                CommandOption::SubCommandGroup(OptionData {
                    name: "user".into(),
                    name_localizations: Default::default(),
                    description: "Get or edit permissions for a user".into(),
                    description_localizations: Default::default(),
                    extra_data: SubCommandGroup {
                        sub_commands: vec![
                            SubCommandOption::SubCommand(OptionData {
                                name: "get".into(),
                                name_localizations: Default::default(),
                                description: "Get permissions for a user".into(),
                                description_localizations: Default::default(),
                                extra_data: SubCommand {
                                    data_options: vec![
                                        CommandDataOption::User(OptionData {
                                            name: "user".into(),
                                            name_localizations: Default::default(),
                                            description: "The user to get".into(),
                                            description_localizations: Default::default(),
                                            extra_data: ParameterData {
                                                required: true,
                                            },
                                        }),
                                        CommandDataOption::Channel(OptionData {
                                            name: "channel".into(),
                                            name_localizations: Default::default(),
                                            description: "The channel permissions to get. If omitted, the guild permissions will be returned".into(),
                                            description_localizations: Default::default(),
                                            extra_data: ChannelData {
                                                required: false,
                                                choices: vec![],
                                            },
                                        }),
                                    ],
                                },
                            }),
                            SubCommandOption::SubCommand(OptionData {
                                name: "edit".into(),
                                name_localizations: Default::default(),
                                description: "Edit permissions for a user".into(),
                                description_localizations: Default::default(),
                                extra_data: SubCommand {
                                    data_options: vec![
                                        CommandDataOption::User(OptionData {
                                            name: "user".into(),
                                            name_localizations: Default::default(),
                                            description: "The user to edit".into(),
                                            description_localizations: Default::default(),
                                            extra_data: ParameterData {
                                                required: true,
                                            },
                                        }),
                                        CommandDataOption::Channel(OptionData {
                                            name: "channel".into(),
                                            name_localizations: Default::default(),
                                            description: "The channel permissions to edit. If omitted, the guild permissions will be edited".into(),
                                            description_localizations: Default::default(),
                                            extra_data: ChannelData {
                                                required: false,
                                                choices: vec![],
                                            },
                                        }),
                                    ],
                                },
                            }),
                        ],
                    },
                }),
                CommandOption::SubCommandGroup(OptionData {
                    name: "role".into(),
                    name_localizations: Default::default(),
                    description: "Get or edit permissions for a role".into(),
                    description_localizations: Default::default(),
                    extra_data: SubCommandGroup {
                        sub_commands: vec![
                            SubCommandOption::SubCommand(OptionData {
                                name: "get".into(),
                                name_localizations: Default::default(),
                                description: "Get permissions for a role".into(),
                                description_localizations: Default::default(),
                                extra_data: SubCommand {
                                    data_options: vec![
                                        CommandDataOption::Role(OptionData {
                                            name: "role".into(),
                                            name_localizations: Default::default(),
                                            description: "The role to get".into(),
                                            description_localizations: Default::default(),
                                            extra_data: ParameterData {
                                                required: true,
                                            },
                                        }),
                                        CommandDataOption::Channel(OptionData {
                                            name: "channel".into(),
                                            name_localizations: Default::default(),
                                            description: "The channel permissions to get. If omitted, the guild permissions will be returned".into(),
                                            description_localizations: Default::default(),
                                            extra_data: ChannelData {
                                                required: false,
                                                choices: vec![],
                                            },
                                        }),
                                    ],
                                },
                            }),
                            SubCommandOption::SubCommand(OptionData {
                                name: "edit".into(),
                                name_localizations: Default::default(),
                                description: "Edit permissions for a role".into(),
                                description_localizations: Default::default(),
                                extra_data: SubCommand {
                                    data_options: vec![
                                        CommandDataOption::Role(OptionData {
                                            name: "role".into(),
                                            name_localizations: Default::default(),
                                            description: "The role to edit".into(),
                                            description_localizations: Default::default(),
                                            extra_data: ParameterData {
                                                required: true,
                                            },
                                        }),
                                        CommandDataOption::Channel(OptionData {
                                            name: "channel".into(),
                                            name_localizations: Default::default(),
                                            description: "The channel permissions to edit. If omitted, the guild permissions will be edited".into(),
                                            description_localizations: Default::default(),
                                            extra_data: ChannelData {
                                                required: false,
                                                choices: vec![],
                                            },
                                        }),
                                    ],
                                },
                            }),
                        ],
                    },
                }),
            ],
        };
        test(CORRECT, command);
    }
}
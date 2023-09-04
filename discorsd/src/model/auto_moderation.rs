use serde_derive::{Deserialize, Serialize};

use crate::model::ids::{ChannelId, GuildId, RoleId, RuleId, UserId};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AutoModRule {
    /// the id of this rule
    pub id: RuleId,
    /// the id of the guild which this rule belongs to
    pub guild_id: GuildId,
    /// the rule name
    pub name: String,
    /// the user which first created this rule
    pub creator_id: UserId,
    /// the rule event type
    pub event_type: EventType,
    /// the rule trigger type & data
    pub trigger_metadata: Trigger,
    /// the actions which will execute when the rule is triggered
    pub actions: Vec<Action>,
    /// whether the rule is enabled
    pub enabled: bool,
    /// the role ids that should not be affected by the rule (Maximum of 20)
    pub exempt_roles: Vec<RoleId>,
    /// the channel ids that should not be affected by the rule (Maximum of 50)
    pub exempt_channels: Vec<ChannelId>,
}

serde_repr! {
    /// Indicates in what event context a rule should be checked.
    pub enum EventType: u8 {
        /// when a member sends or edits a message in the guild
        MessageSend = 1,
    }
}

serde_num_tag! {
    #[derive(Clone, Debug)]
    pub enum Trigger = "trigger_type": u8 as TriggerType {
        /// check if content contains words from a user defined list of keywords
        ///
        /// Max 6 per guild
        (1) = Keyword {
            /// substrings which will be searched for in content (Maximum of 1000).
            ///
            /// A keyword can be a phrase which contains multiple words. Wildcard symbols can be
            /// used to customize how each keyword will be matched. Each keyword must be 60
            /// characters or less.
            #serde = default
            keyword_filter: Vec<String>,
            /// regular expression patterns which will be matched against content (Maximum of 10).
            #serde = default
            regex_patterns: Vec<String>,
            /// substrings which will be exempt from triggering the preset trigger type (Maximum of
            /// 100).
            ///
            /// Each allow_list keyword can be a phrase which contains multiple words. Wildcard
            /// symbols can be used to customize how each keyword will be matched.
            #serde = default
            allow_list: Vec<String>,
        },
        /// check if content represents generic spam
        ///
        /// Max 1 per guild
        (3) = Spam,
        /// check if content contains words from internal pre-defined wordsets
        ///
        /// Max 1 per guild
        (4) = KeywordPreset {
            /// the internally pre-defined wordsets which will be searched for in content
            presets: Vec<KeywordPreset>,
            /// substrings which will be exempt from triggering the preset trigger type (Maximum of
            /// 1000).
            ///
            /// Each allow_list keyword can be a phrase which contains multiple words. Wildcard
            /// symbols can be used to customize how each keyword will be matched.
            allow_list: Vec<String>,
        },
        /// check if content contains more unique mentions than allowed
        ///
        /// Max 1 per guild
        (5) = MentionSpam {
            /// total number of unique role and user mentions allowed per message (Maximum of 50)
            mention_total_limit: u32,
            /// whether to automatically detect mention raids
            mention_raid_protection_enabled: bool,
        },
    }
}

// mod rule_serde {
//     use std::borrow::Cow;
//
//     use serde::{Deserialize, Deserializer, Serialize, Serializer};
//     use serde::de::{Error, Unexpected};
//
//     use crate::model::ids::{ChannelId, GuildId, RoleId, RuleId, UserId};
//
//     use super::{Action, AutoModRule, EventType, KeywordPreset, Trigger};
//
//     #[derive(Deserialize, Serialize)]
//     pub(super) struct RawAutoModRule<'a> {
//         id: RuleId,
//         guild_id: GuildId,
//         name: &'a str,
//         creator_id: UserId,
//         event_type: EventType,
//         trigger_type: u8,
//         trigger_metadata: Metadata<'a>,
//         actions: Cow<'a, [Action]>,
//         enabled: bool,
//         exempt_roles: Cow<'a, [RoleId]>,
//         exempt_channels: Cow<'a, [ChannelId]>,
//     }
//
//     fn cow_empty<T>(c: &Cow<[T]>) -> bool where [T]: ToOwned {
//         c.is_empty()
//     }
//
//     #[derive(Deserialize, Serialize, Default)]
//     pub(super) struct Metadata<'a> {
//         #[serde(default, skip_serializing_if = "cow_empty")]
//         keyword_filter: Cow<'a, [String]>,
//         #[serde(default, skip_serializing_if = "cow_empty")]
//         presets: Cow<'a, [KeywordPreset]>,
//         #[serde(default, skip_serializing_if = "cow_empty")]
//         allow_list: Cow<'a, [String]>,
//         #[serde(skip_serializing_if = "Option::is_none")]
//         mention_total_limit: Option<u32>,
//     }
//
//     impl Serialize for AutoModRule {
//         fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
//             let default = Metadata::default();
//             let (trigger_type, trigger_metadata) = match &self.trigger {
//                 Trigger::Keyword { keyword_filter } => (1, Metadata {
//                     keyword_filter: keyword_filter.into(),
//                     ..default
//                 }),
//                 Trigger::Spam => (3, default),
//                 Trigger::KeywordPreset { presets, allow_list } => (4, Metadata {
//                     presets: presets.into(),
//                     allow_list: allow_list.into(),
//                     ..default
//                 }),
//                 &Trigger::MentionSpam { mention_total_limit } => (5, Metadata {
//                     mention_total_limit: Some(mention_total_limit),
//                     ..default
//                 }),
//             };
//             println!("self.actions = {:?}", self.actions);
//             RawAutoModRule {
//                 id: self.id,
//                 guild_id: self.guild,
//                 name: &self.name,
//                 creator_id: self.creator,
//                 event_type: self.event_type,
//                 trigger_type,
//                 trigger_metadata,
//                 actions: (&self.actions).into(),
//                 enabled: self.enabled,
//                 exempt_roles: (&self.exempt_roles).into(),
//                 exempt_channels: (&self.exempt_channels).into(),
//             }.serialize(s)
//         }
//     }
//
//     impl<'de> Deserialize<'de> for AutoModRule {
//         fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
//             let RawAutoModRule {
//                 id,
//                 guild_id,
//                 name,
//                 creator_id,
//                 event_type,
//                 trigger_type,
//                 trigger_metadata: Metadata { keyword_filter, presets, allow_list, mention_total_limit },
//                 actions,
//                 enabled,
//                 exempt_roles,
//                 exempt_channels
//             } = RawAutoModRule::deserialize(d)?;
//             let trigger = match trigger_type {
//                 1 => Trigger::Keyword { keyword_filter: keyword_filter.into() },
//                 3 => Trigger::Spam,
//                 4 => Trigger::KeywordPreset { presets: presets.into(), allow_list: allow_list.into() },
//                 5 => mention_total_limit.map(|mention_total_limit| Trigger::MentionSpam { mention_total_limit })
//                     .ok_or_else(|| D::Error::missing_field("Trigger::MentionSpam::mention_total_limit"))?,
//                 unknown => return Err(D::Error::invalid_value(Unexpected::Unsigned(unknown as _), &"1, 3, 4, 5")),
//             };
//             println!("actions = {:?}", actions);
//             Ok(Self {
//                 id,
//                 guild: guild_id,
//                 name: name.to_string(),
//                 creator: creator_id,
//                 event_type,
//                 trigger,
//                 actions: actions.to_vec(),
//                 enabled,
//                 exempt_roles: exempt_roles.to_vec(),
//                 exempt_channels: exempt_channels.to_vec(),
//             })
//         }
//     }
// }

serde_repr! {
    pub enum KeywordPreset: u8 {
        /// Words that may be considered forms of swearing or cursing
		Profanity = 1,
        /// Words that refer to sexually explicit behavior or activity
		SexualContent = 2,
        /// Personal insults or words that may be considered hate speech
		Slurs = 3,
    }
}

serde_num_tag! {
    /// An action which will execute whenever a rule is triggered.
    #[derive(Debug, Clone)]
    pub enum Action = "type": u8 as ActionType {
        /// blocks a member's message and prevents it from being posted. A custom explanation can be
        /// specified and shown to members whenever their message is blocked.
        (1) = BlockMessage {
            /// additional metadata needed during execution for this specific action type
            #serde = default
            #serde = skip_serializing_if = "Option::is_none"
            metadata: Option<BlockMessage>,
        },
        /// logs user content to a specified channel
        (2) = SendAlertMessage {
            /// additional metadata needed during execution for this specific action type
            metadata: SendAlertMessage,
        },
        /// timeout user for a specified duration
        ///
        /// A TIMEOUT action can only be set up for KEYWORD and MENTION_SPAM rules. The MODERATE_MEMBERS
        /// permission is required to use the TIMEOUT action type.
        (3) = Timeout {
            /// additional metadata needed during execution for this specific action type
            metadata: Timeout,
        },
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct BlockMessage {
    /// additional explanation that will be shown to members whenever their message is blocked
    ///
    /// maximum of 150 characters
    custom_message: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub struct SendAlertMessage {
    /// channel to which user content should be logged
    channel_id: ChannelId,
}

#[derive(Deserialize, Serialize, Clone, Copy, Debug)]
pub struct Timeout {
    /// timeout duration in seconds
    ///
    /// Maximum of 2419200 seconds (4 weeks)
    duration_seconds: u32,
}

#[cfg(test)]
mod automod_tests {
    use super::*;

    fn assert(json: &str) {
        // todo fix this so it doesn't error
        let rule: AutoModRule = serde_json::from_str(json).unwrap();
        println!("rule = {:#?}", rule);
        let back = serde_json::to_string_pretty(&rule).unwrap();
        assert_eq!(json, back);
    }

    #[test]
    fn automod_rule() {
        assert(r#"{
  "id": "969707018069872670",
  "guild_id": "613425648685547541",
  "name": "Keyword Filter 1",
  "creator_id": "423457898095789043",
  "trigger_type": 1,
  "event_type": 1,
  "actions": [
    {
      "type": 1,
      "metadata": { "custom_message": "Please keep financial discussions limited to the #finance channel" }
    },
    {
      "type": 2,
      "metadata": { "channel_id": "123456789123456789" }
    },
    {
      "type": 3,
      "metadata": { "duration_seconds": 60 }
    }
  ],
  "trigger_metadata": {
    "keyword_filter": ["cat*", "*dog", "*ana*", "i like c++"],
    "regex_patterns": ["(b|c)at", "^(?:[0-9]{1,3}\\.){3}[0-9]{1,3}$"]
  },
  "enabled": true,
  "exempt_roles": ["323456789123456789", "423456789123456789"],
  "exempt_channels": ["523456789123456789"]
}"#)
    }

    #[test]
    fn asdsa() {
        let rule = Action::BlockMessage {
            metadata: None,
        };
        let string = serde_json::to_string_pretty(&rule).unwrap();
        println!("{string}");
    }

    #[test]
    fn kja() {
        let json = r#"{
      "type": 1,
      "metadata": {}
    }"#;
        let action: Action = serde_json::from_str(json).unwrap();
        println!("action = {:#?}", action);
    }
}
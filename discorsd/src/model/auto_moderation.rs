use crate::model::ids::{ChannelId, GuildId, RoleId, RuleId, UserId};

#[derive(Debug, Clone)]
pub struct AutoModRule {
    /// the id of this rule
    pub id: RuleId,
    /// the id of the guild which this rule belongs to
    pub guild: GuildId,
    /// the rule name
    pub name: String,
    /// the user which first created this rule
    pub creator: UserId,
    /// the rule event type
    pub event_type: EventType,
    /// the rule trigger type & data
    pub trigger: Trigger,
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

serde_repr! {
    pub enum TriggerType: u8 {
        /// check if content contains words from a user defined list of keywords
        Keyword = 1,
        /// check if content represents generic spam
        Spam = 3,
        /// check if content contains words from internal pre-defined wordsets
        KeywordPreset = 4,
        /// check if content contains more unique mentions than allowed
        MentionSpam = 5,
    }
}

#[derive(Clone, Debug)]
pub enum Trigger {
    /// check if content contains words from a user defined list of keywords
    ///
    /// Max 3 per guild
    Keyword {
        /// substrings which will be searched for in content
        keyword_filter: Vec<String>,
    },
    /// check if content represents generic spam
    ///
    /// Max 1 per guild
    Spam,
    /// check if content contains words from internal pre-defined wordsets
    ///
    /// Max 1 per guild
    KeywordPreset {
        /// the internally pre-defined wordsets which will be searched for in content
        presets: Vec<KeywordPreset>,
        /// substrings which will be exempt from triggering the preset trigger type
        allow_list: Vec<String>,
    },
    /// check if content contains more unique mentions than allowed
    ///
    /// Max 1 per guild
    MentionSpam {
        /// total number of unique role and user mentions allowed per message (Maximum of 50)
        mention_total_limit: u32,
    },
}

mod rule_serde {
    use std::borrow::Cow;

    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde::de::{Error, Unexpected};

    use crate::model::ids::{ChannelId, GuildId, RoleId, RuleId, UserId};

    use super::{Action, AutoModRule, EventType, KeywordPreset, Trigger};

    #[derive(Deserialize, Serialize)]
    pub(super) struct RawAutoModRule<'a> {
        id: RuleId,
        guild_id: GuildId,
        name: &'a str,
        creator_id: UserId,
        event_type: EventType,
        trigger_type: u8,
        trigger_metadata: Metadata<'a>,
        actions: Cow<'a, [Action]>,
        enabled: bool,
        exempt_roles: Cow<'a, [RoleId]>,
        exempt_channels: Cow<'a, [ChannelId]>,
    }

    fn cow_empty<T>(c: &Cow<[T]>) -> bool where [T]: ToOwned {
        c.is_empty()
    }

    #[derive(Deserialize, Serialize, Default)]
    pub(super) struct Metadata<'a> {
        #[serde(default, skip_serializing_if = "cow_empty")]
        keyword_filter: Cow<'a, [String]>,
        #[serde(default, skip_serializing_if = "cow_empty")]
        presets: Cow<'a, [KeywordPreset]>,
        #[serde(default, skip_serializing_if = "cow_empty")]
        allow_list: Cow<'a, [String]>,
        #[serde(skip_serializing_if = "Option::is_none")]
        mention_total_limit: Option<u32>,
    }

    impl Serialize for AutoModRule {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let default = Metadata::default();
            let (trigger_type, trigger_metadata) = match &self.trigger {
                Trigger::Keyword { keyword_filter } => (1, Metadata {
                    keyword_filter: keyword_filter.into(),
                    ..default
                }),
                Trigger::Spam => (3, default),
                Trigger::KeywordPreset { presets, allow_list } => (4, Metadata {
                    presets: presets.into(),
                    allow_list: allow_list.into(),
                    ..default
                }),
                &Trigger::MentionSpam { mention_total_limit } => (5, Metadata {
                    mention_total_limit: Some(mention_total_limit),
                    ..default
                }),
            };
            RawAutoModRule {
                id: self.id,
                guild_id: self.guild,
                name: &self.name,
                creator_id: self.creator,
                event_type: self.event_type,
                trigger_type,
                trigger_metadata,
                actions: (&self.actions).into(),
                enabled: self.enabled,
                exempt_roles: (&self.exempt_roles).into(),
                exempt_channels: (&self.exempt_channels).into(),
            }.serialize(s)
        }
    }

    impl<'de> Deserialize<'de> for AutoModRule {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let RawAutoModRule {
                id,
                guild_id,
                name,
                creator_id,
                event_type,
                trigger_type,
                trigger_metadata: Metadata { keyword_filter, presets, allow_list, mention_total_limit },
                actions,
                enabled,
                exempt_roles,
                exempt_channels
            } = RawAutoModRule::deserialize(d)?;
            let trigger = match trigger_type {
                1 => Trigger::Keyword { keyword_filter: keyword_filter.into() },
                3 => Trigger::Spam,
                4 => Trigger::KeywordPreset { presets: presets.into(), allow_list: allow_list.into() },
                5 => mention_total_limit.map(|mention_total_limit| Trigger::MentionSpam { mention_total_limit })
                    .ok_or_else(|| D::Error::missing_field("Trigger::MentionSpam::mention_total_limit"))?,
                unknown => return Err(D::Error::invalid_value(Unexpected::Unsigned(unknown as _), &"1, 3, 4, 5")),
            };
            Ok(Self {
                id,
                guild: guild_id,
                name: name.to_string(),
                creator: creator_id,
                event_type,
                trigger,
                actions: actions.to_vec(),
                enabled,
                exempt_roles: exempt_roles.to_vec(),
                exempt_channels: exempt_channels.to_vec(),
            })
        }
    }
}

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

/// An action which will execute whenever a rule is triggered.
#[derive(Debug, Clone, Copy)]
pub enum Action {
    /// blocks the content of a message according to the rule
    BlockMessage,
    /// logs user content to a specified channel
    SendAlertMessage {
        /// channel to which user content should be logged
        channel: ChannelId,
    },
    /// timeout user for a specified duration
    ///
    /// A TIMEOUT action can only be set up for KEYWORD and MENTION_SPAM rules. The MODERATE_MEMBERS
    /// permission is required to use the TIMEOUT action type.
    Timeout {
        /// timeout duration in seconds
        ///
        /// Maximum of 2419200 seconds (4 weeks)
        duration: u32,
    },
}

mod action_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde::de::{Error, Unexpected};

    use crate::model::ids::ChannelId;

    use super::Action;

    #[derive(Serialize, Deserialize)]
    pub(super) struct RawAction {
        #[serde(rename = "type")]
        kind: u8,
        metadata: Metadata,
    }

    #[derive(Serialize, Deserialize, Default)]
    pub(super) struct Metadata {
        #[serde(skip_serializing_if = "Option::is_none")]
        channel_id: Option<ChannelId>,
        #[serde(skip_serializing_if = "Option::is_none")]
        duration_seconds: Option<u32>,
    }

    impl Serialize for Action {
        fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            let default = Default::default();
            match self {
                Self::BlockMessage => RawAction { kind: 1, metadata: default },
                &Self::SendAlertMessage { channel } => RawAction {
                    kind: 2,
                    metadata: Metadata { channel_id: Some(channel), ..default },
                },
                &Self::Timeout { duration } => RawAction {
                    kind: 3,
                    metadata: Metadata { duration_seconds: Some(duration), ..default },
                },
            }.serialize(s)
        }
    }

    impl<'de> Deserialize<'de> for Action {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            let RawAction { kind, metadata: Metadata { channel_id, duration_seconds } } = RawAction::deserialize(d)?;
            match kind {
                1 => Ok(Self::BlockMessage),
                2 => channel_id.map(|channel| Self::SendAlertMessage { channel })
                    .ok_or_else(|| D::Error::missing_field("Action::SendAlertMessage::channel_id")),
                3 => duration_seconds.map(|duration| Self::Timeout { duration })
                    .ok_or_else(|| D::Error::missing_field("Action::Timeout::duration_seconds")),
                unknown => Err(D::Error::invalid_value(Unexpected::Unsigned(unknown as _), &"1, 2, or 3")),
            }
        }
    }
}

#[cfg(test)]
mod automod_tests {
    use super::*;

    fn assert(json: &str) {
        let rule: AutoModRule = serde_json::from_str(json).unwrap();
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
  "event_type": 1,
  "trigger_type": 1,
  "trigger_metadata": {
    "keyword_filter": [
      "cat*",
      "*dog",
      "*ana*",
      "i like javascript"
    ]
  },
  "actions": [
    {
      "type": 1,
      "metadata": {}
    },
    {
      "type": 2,
      "metadata": {
        "channel_id": "123456789123456789"
      }
    }
  ],
  "enabled": true,
  "exempt_roles": [
    "323456789123456789",
    "423456789123456789"
  ],
  "exempt_channels": [
    "523456789123456789"
  ]
}"#)
    }
}
use chrono::{DateTime, Utc};
use serde_derive::{Deserialize, Serialize};

use crate::model::ids::*;
pub use crate::model::ids::ChannelId;
use crate::model::permissions::Permissions;
use crate::model::user::User;
use crate::model::voice::VoiceRegion;

serde_num_tag! {
    /// Represents a guild or DM channel within Discord.
    #[derive(Debug, Clone)]
    pub enum Channel = "type": u8 as ChannelType {
        /// a text channel within a server
        (0) = Text(TextChannel),
        /// a direct message between users
        (1) = Dm(DmChannel),
        /// a voice channel within a server
        (2) = Voice(VoiceChannel),
        /// a direct message between multiple users
        (3) = GroupDm(GroupDmChannel),
        /// an [organizational category](https://support.discord.com/hc/en-us/articles/115001580171-Channel-Categories-101)
        /// that contains up to 50 channels
        (4) = Category(CategoryChannel),
        /// a channel that [users can follow and crosspost into their own server](https://support.discord.com/hc/en-us/articles/360032008192)
        (5) = Announcement(AnnouncementChannel),
        // /// a channel in which game developers can
        // /// [sell their game on Discord](https://discord.com/developers/docs/game-and-server-management/special-channels)
        // Store(StoreChannel),
        /// a temporary sub-channel within a GUILD_ANNOUNCEMENT channel
        (10) = AnnouncementThread(Thread),
        /// a temporary sub-channel within a GUILD_TEXT channel
        (11) = PublicThread(Thread),
        /// a temporary sub-channel within a GUILD_TEXT channel that is only viewable by those invited and those with the MANAGE_THREADS permission
        (12) = PrivateThread(Thread),
        /// a voice channel for hosting events with an audience
        (13) = GuildStageVoice(GuildStageVoice),
        /// the channel in a hub containing the listed servers
        (14) = GuildDirectory(GuildDirectory),
        /// Channel that can only contain threads
        (15) = GuildForum(GuildForum),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test(json: &'static str) -> Channel {
        match serde_json::from_str(json) {
            Ok(channel) => channel,
            Err(e) => panic!("{e}"),
        }
    }

    #[test]
    fn text_channels_category() {
        const JSON: &str = r#"{"version": 0,"type": 4,"position": 0,"permission_overwrites": [],"name": "Text Channels","id": "492122906864779275","flags": 0}"#;
        println!("channel = {:?}", test(JSON));
    }
    
    #[test]
    fn general_text() {
        const JSON: &str = r#"{"version":0,"type":0,"topic":null,"rate_limit_per_user":0,"position":0,"permission_overwrites":[{"type":0,"id":"492122906864779274","deny":"0","allow":"0"}],"parent_id":"492122906864779275","name":"general","last_message_id":"991036430912454696","id":"492122906864779276","flags":0}"#;
        println!("channel = {:?}", test(JSON));
    }
}

impl Channel {
    pub const fn guild_id(&self) -> Option<GuildId> {
        match self {
            Self::Text(t) => t.guild_id,
            Self::Voice(v) => v.guild_id,
            Self::Category(c) => c.guild_id,
            Self::Announcement(n) => n.guild_id,
            Self::Dm(_) | Self::GroupDm(_) => None,
            Self::AnnouncementThread(a) => a.guild_id,
            Self::PublicThread(t) => t.guild_id,
            Self::PrivateThread(t) => t.guild_id,
            Self::GuildStageVoice(v) => v.guild_id,
            Self::GuildDirectory(d) => d.guild_id,
            Self::GuildForum(f) => f.guild_id,
        }
    }

    pub const fn text(&self) -> Option<&TextChannel> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn into_text(self) -> Option<TextChannel> {
        match self {
            Self::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn overwrites(&self) -> Option<&[Overwrite]> {
        match self {
            Self::Text(t) => Some(&t.permission_overwrites),
            Self::Voice(v) => Some(&v.permission_overwrites),
            Self::Category(c) => Some(&c.permission_overwrites),
            Self::Announcement(n) => Some(&n.permission_overwrites),
            // Self::Store(s) => Some(&s.permission_overwrites),
            Self::Dm(_) | Self::GroupDm(_) => None,
            // todo
            Self::AnnouncementThread(_) => None,
            Self::PublicThread(_) => None,
            Self::PrivateThread(_) => None,
            Self::GuildStageVoice(_) => None,
            Self::GuildDirectory(_) => None,
            Self::GuildForum(_) => None,
        }
    }
}

id_eq!(Channel);
impl Id for Channel {
    type Id = ChannelId;

    fn id(&self) -> ChannelId {
        match self {
            Self::Text(c) => c.id,
            Self::Dm(c) => c.id,
            Self::Voice(c) => c.id,
            Self::GroupDm(c) => c.id,
            Self::Category(c) => c.id,
            Self::Announcement(c) => c.id,
            // Self::Store(c) => c.id,
            Self::AnnouncementThread(c) => c.id,
            Self::PublicThread(c) => c.id,
            Self::PrivateThread(c) => c.id,
            Self::GuildStageVoice(c) => c.id,
            Self::GuildDirectory(c) => c.id,
            Self::GuildForum(c) => c.id,
        }
    }
}

/// a text channel within a server
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TextChannel {
    /// the id of this channel
    pub id: ChannelId,
    /// the id of the guild
    pub guild_id: Option<GuildId>,
    /// the name of the channel (2-100 characters)
    pub name: String,
    /// sorting position of the channel
    pub position: u32,
    /// explicit permission overwrites for members and roles
    pub permission_overwrites: Vec<Overwrite>,
    /// amount of seconds a user has to wait before sending another message (0-21600); bots, as well
    /// as users with the permission `manage_messages` or `manage_channel`, are unaffected
    pub rate_limit_per_user: Option<u32>,
    #[serde(default)]
    /// whether the channel is nsfw
    pub nsfw: bool,
    /// the channel topic (0-1024 characters)
    pub topic: Option<String>,
    /// the id of the last message sent in this channel (may not point to an existing or valid message)
    pub last_message_id: Option<MessageId>,
    /// id of the parent category for a channel (each parent category can contain up to 50 channels)
    pub parent_id: Option<ChannelId>,
    /// when the last pinned message was pinned. This may be `None` in events such as `GUILD_CREATE`
    /// when a message is not pinned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_pin_timestamp: Option<DateTime<Utc>>,
    /// default duration, copied onto newly created threads, in minutes, threads will stop showing
    /// in the channel list after the specified period of inactivity
    pub default_auto_archive_duration: Option<ThreadArchiveDuration>,
    /// computed permissions for the invoking user in the channel, including overwrites, only included when part of the resolved data received on a slash command interaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
    /// the initial rate_limit_per_user to set on newly created threads in a channel. this field is copied to the thread at creation time and does not live update.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_thread_rate_limit_per_user: Option<u32>,
}

id_impl!(TextChannel => id: ChannelId);

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct ChannelFlags: u8 {
        /// this thread is pinned to the top of its parent GUILD_FORUM channel
	    const PINNED = 1 << 1;
        /// whether a tag is required to be specified when creating a thread in a GUILD_FORUM channel. Tags are specified in the applied_tags field.
	    const REQUIRE_TAG = 1 << 4;
    }
}
serde_bitflag!(ChannelFlags: u8);

impl From<TextChannel> for Channel {
    fn from(c: TextChannel) -> Self {
        Self::Text(c)
    }
}

/// a direct message between users
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct DmChannel {
    /// the id of this channel
    pub id: ChannelId,
    /// the id of the last message sent in this channel (may not point to an existing or valid message)
    pub last_message_id: Option<MessageId>,
    /// the recipients of the DM
    #[serde(rename = "recipients", with = "one_recipient")]
    pub recipient: User,
    /// when the last pinned message was pinned. This may be `None` in events such as `GUILD_CREATE`
    /// when a message is not pinned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_pin_timestamp: Option<DateTime<Utc>>,
    /// computed permissions for the invoking user in the channel, including overwrites, only included when part of the resolved data received on a slash command interaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
}

id_impl!(DmChannel => id: ChannelId);

impl From<DmChannel> for Channel {
    fn from(c: DmChannel) -> Self {
        Self::Dm(c)
    }
}

mod one_recipient {
    use serde::{Deserialize, Deserializer, Serializer};
    use serde::ser::SerializeSeq;

    use crate::model::User;

    pub fn serialize<S: Serializer>(recipient: &User, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(1))?;
        seq.serialize_element(recipient)?;
        seq.end()
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<User, D::Error> {
        let [id] = <[User; 1]>::deserialize(d)?;
        Ok(id)
    }
}

/// a voice channel within a server
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VoiceChannel {
    /// the id of this channel
    pub id: ChannelId,
    /// the id of the guild
    pub guild_id: Option<GuildId>,
    /// sorting position of the channel
    pub position: u32,
    /// the name of the channel (2-100 characters)
    pub name: String,
    /// id of the parent category for a channel (each parent category can contain up to 50 channels)
    pub parent_id: Option<ChannelId>,
    /// the bitrate (in bits) of the voice channel
    pub bitrate: u32,
    /// the id of the last message sent in this channel (may not point to an existing or valid message)
    pub last_message_id: Option<MessageId>,
    /// the user limit of the voice channel
    pub user_limit: u32,
    /// voice region id for the voice channel, automatic when set to null
    pub rtc_region: Option<VoiceRegion>,
    /// explicit permission overwrites for members and roles
    pub permission_overwrites: Vec<Overwrite>,
    /// amount of seconds a user has to wait before sending another message (0-21600); bots, as well
    /// as users with the permission `manage_messages` or `manage_channel`, are unaffected
    pub rate_limit_per_user: Option<u32>,
    /// the camera video quality mode of the voice channel, 1 when not present
    /// whether the channel is nsfw
    #[serde(default)]
    pub nsfw: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video_quality_mode: Option<VideoQualityMode>,
}

id_impl!(VoiceChannel => id: ChannelId);

impl From<VoiceChannel> for Channel {
    fn from(c: VoiceChannel) -> Self {
        Self::Voice(c)
    }
}

serde_repr! {
    pub enum VideoQualityMode: u8 {
        /// Discord chooses the quality for optimal performance
        AUTO = 1,
        /// 720p
        FULL = 2,
    }
}

/// a direct message between multiple users
///
/// bots cannot be in these channels
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GroupDmChannel {
    /// the id of this channel
    pub id: ChannelId,
    /// the name of the channel (2-100 characters)
    pub name: String,
    /// icon hash
    pub icon: Option<String>,
    /// the recipients of the DM
    pub recipients: Vec<User>,
    /// the id of the last message sent in this channel (may not point to an existing or valid message)
    pub last_message_id: Option<MessageId>,
    /// id of the DM creator
    pub owner_id: UserId,
    /// application id of the group DM creator if it is bot-created
    #[serde(skip_serializing_if = "Option::is_none")]
    pub application_id: Option<ApplicationId>,
    /// when the last pinned message was pinned. This may be `None` in events such as `GUILD_CREATE`
    /// when a message is not pinned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_pin_timestamp: Option<DateTime<Utc>>,
    /// computed permissions for the invoking user in the channel, including overwrites, only included when part of the resolved data received on a slash command interaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
}

id_impl!(GroupDmChannel => id: ChannelId);

impl From<GroupDmChannel> for Channel {
    fn from(c: GroupDmChannel) -> Self {
        Self::GroupDm(c)
    }
}

/// an [organizational category](https://support.discord.com/hc/en-us/articles/115001580171-Channel-Categories-101)
/// within a server that contains up to 50 channels
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct CategoryChannel {
    /// the id of this channel
    pub id: ChannelId,
    /// the id of the guild
    pub guild_id: Option<GuildId>,
    /// the name of the channel (2-100 characters)
    pub name: String,
    /// explicit permission overwrites for members and roles
    pub permission_overwrites: Vec<Overwrite>,
    /// if this category is nsfw
    #[serde(default)]
    pub nsfw: bool,
    /// sorting position of the channel
    pub position: u32,
}

id_impl!(CategoryChannel => id: ChannelId);

impl From<CategoryChannel> for Channel {
    fn from(c: CategoryChannel) -> Self {
        Self::Category(c)
    }
}

/// A channel that [users can follow and crosspost into their own server](https://support.discord.com/hc/en-us/articles/360032008192).
///
/// Bots can post or publish messages in this type of channel if they have the proper permissions.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AnnouncementChannel {
    /// the id of this channel
    pub id: ChannelId,
    /// the id of the guild
    pub guild_id: Option<GuildId>,
    /// the name of the channel (2-100 characters)
    pub name: String,
    /// sorting position of the channel
    pub position: u32,
    /// explicit permission overwrites for members and roles
    pub permission_overwrites: Vec<Overwrite>,
    /// whether the channel is nsfw
    #[serde(default)]
    pub nsfw: bool,
    /// the channel topic (0-1024 characters)
    pub topic: Option<String>,
    /// the id of the last message sent in this channel (may not point to an existing or valid message)
    pub last_message_id: Option<MessageId>,
    /// id of the parent category for a channel (each parent category can contain up to 50 channels)
    pub parent_id: Option<ChannelId>,
    /// when the last pinned message was pinned. This may be `None` in events such as `GUILD_CREATE`
    /// when a message is not pinned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_pin_timestamp: Option<DateTime<Utc>>,
    /// default duration, copied onto newly created threads, in minutes, threads will stop showing
    /// in the channel list after the specified period of inactivity
    pub default_auto_archive_duration: Option<ThreadArchiveDuration>,
    /// computed permissions for the invoking user in the channel, including overwrites, only included when part of the resolved data received on a slash command interaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
    /// the initial rate_limit_per_user to set on newly created threads in a channel. this field is copied to the thread at creation time and does not live update.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_thread_rate_limit_per_user: Option<u32>,
}

id_impl!(AnnouncementChannel => id: ChannelId);

impl From<AnnouncementChannel> for Channel {
    fn from(c: AnnouncementChannel) -> Self {
        Self::Announcement(c)
    }
}

/// A channel that [users can follow and crosspost into their own server](https://support.discord.com/hc/en-us/articles/360032008192).
///
/// Bots can post or publish messages in this type of channel if they have the proper permissions.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Thread {
    /// the id of this channel
    pub id: ChannelId,
    /// the id of the guild
    pub guild_id: Option<GuildId>,
    /// id of the text channel this thread was created in
    pub parent_id: Option<ChannelId>,
    /// id of the thread creator
    pub owner_id: UserId,
    /// the name of the channel (2-100 characters)
    pub name: String,
    /// the id of the last message sent in this channel (may not point to an existing or valid message)
    pub last_message_id: Option<MessageId>,
    /// number of messages, (not including the initial message or deleted messages) in a thread.
    pub message_count: usize,
    /// an approximate count of users in a thread, stops counting at 50
    pub member_count: u8,
    /// amount of seconds a user has to wait before sending another message (0-21600); bots, as well
    /// as users with the permission `manage_messages` or `manage_channel`, are unaffected
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_per_user: Option<u32>,
    /// when the last pinned message was pinned. This may be `None` in events such as `GUILD_CREATE`
    /// when a message is not pinned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_pin_timestamp: Option<DateTime<Utc>>,
    /// thread-specific fields not needed by other channels
    pub thread_metadata: ThreadMetadata,
    /// thread member object for the current user, if they have joined the thread, only included on certain API endpoints
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member: Option<ThreadMember>,
    /// computed permissions for the invoking user in the channel, including overwrites, only included when part of the resolved data received on a slash command interaction
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Permissions>,
    /// number of messages ever sent in a thread, it's similar to message_count on message creation, but will not decrement the number when a message is deleted
    pub total_message_sent: usize,
}

id_impl!(Thread => id: ChannelId);

impl From<Thread> for Channel {
    fn from(c: Thread) -> Self {
        Self::AnnouncementThread(c)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ThreadMetadata {
    /// whether the thread is archived
    pub archived: bool,
    /// the thread will stop showing in the channel list after auto_archive_duration minutes of
    /// inactivity
    pub auto_archive_duration: ThreadArchiveDuration,
    /// timestamp when the thread's archive status was last changed, used for calculating recent
    /// activity
    pub archive_timestamp: DateTime<Utc>,
    /// whether the thread is locked; when a thread is locked, only users with MANAGE_THREADS can
    /// unarchive it
    pub locked: bool,
    /// whether non-moderators can add other non-moderators to a thread; only available on private
    /// threads
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invitable: Option<bool>,
    /// timestamp when the thread was created; only populated for threads created after 2022-01-09
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_timestamp: Option<DateTime<Utc>>,
    /// channel flags
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<ChannelFlags>,
    /// the IDs of the set of tags that have been applied to a thread in a GUILD_FORUM channel
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applied_tags: Vec<TagId>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ThreadMember {
    /// the id of the thread
    pub id: Option<ChannelId>,
    /// the id of the user
    pub user_id: Option<UserId>,
    /// the time the current user last joined the thread
    pub join_timestamp: DateTime<Utc>,
    /// any user-thread settings, currently only used for notifications
    pub flags: u32,
}

serde_repr! {
    pub enum ThreadArchiveDuration: u16 {
        Hour = 60,
        Day = 1440,
        ThreeDays = 4320,
        Week = 10080,
    }
}

/// a voice channel within a server
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GuildStageVoice {
    /// the id of this channel
    pub id: ChannelId,
    /// the id of the guild
    pub guild_id: Option<GuildId>,
    /// sorting position of the channel
    pub position: u32,
    /// explicit permission overwrites for members and roles
    pub permission_overwrites: Vec<Overwrite>,
    /// the name of the channel (2-100 characters)
    pub name: String,
    /// the bitrate (in bits) of the voice channel
    pub bitrate: u32,
    /// the user limit of the voice channel
    pub user_limit: u32,
    /// id of the parent category for a channel (each parent category can contain up to 50 channels)
    pub parent_id: Option<ChannelId>,
    /// voice region id for the voice channel, automatic when set to null
    pub rtc_region: Option<VoiceRegion>,
    /// the camera video quality mode of the voice channel, 1 when not present
    pub video_quality_mode: VideoQualityMode,
}

id_impl!(GuildStageVoice => id: ChannelId);

impl From<GuildStageVoice> for Channel {
    fn from(c: GuildStageVoice) -> Self {
        Self::GuildStageVoice(c)
    }
}

/// a voice channel within a server
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GuildDirectory {
    /// the id of this channel
    pub id: ChannelId,
    /// the id of the guild
    pub guild_id: Option<GuildId>,
    /// sorting position of the channel
    pub position: u32,
    /// explicit permission overwrites for members and roles
    pub permission_overwrites: Vec<Overwrite>,
    /// the name of the channel (2-100 characters)
    pub name: String,
    /// id of the parent category for a channel (each parent category can contain up to 50 channels)
    pub parent_id: Option<ChannelId>,
}

id_impl!(GuildDirectory => id: ChannelId);

impl From<GuildDirectory> for Channel {
    fn from(c: GuildDirectory) -> Self {
        Self::GuildDirectory(c)
    }
}

/// a voice channel within a server
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GuildForum {
    /// the id of this channel
    pub id: ChannelId,
    /// the id of the guild
    pub guild_id: Option<GuildId>,
    /// sorting position of the channel
    pub position: u32,
    /// the name of the channel (2-100 characters)
    pub name: String,
    /// id of the parent category for a channel (each parent category can contain up to 50 channels)
    pub parent_id: Option<ChannelId>,
    /// the channel topic (0-1024 characters)
    pub topic: Option<String>,
    /// default duration, copied onto newly created threads, in minutes, threads will stop showing
    /// in the channel list after the specified period of inactivity
    pub default_auto_archive_duration: Option<ThreadArchiveDuration>,
    /// channel flags
    pub flags: Option<ChannelFlags>,
    /// the set of tags that can be used in a GUILD_FORUM channel
    pub available_tags: Vec<Tag>,
    /// the emoji to show in the add reaction button on a thread in a GUILD_FORUM channel
    pub default_reaction_emoji: Option<TagEmoji>,
    /// the initial rate_limit_per_user to set on newly created threads in a channel. this field is
    /// copied to the thread at creation time and does not live update.
    pub default_thread_rate_limit_per_user: u32,
    /// the default sort order type used to order posts in GUILD_FORUM channels. Defaults to null,
    /// which indicates a preferred sort order hasn't been set by a channel admin
    pub default_sort_order: Option<SortOrder>,
}

id_impl!(GuildForum => id: ChannelId);

impl From<GuildForum> for Channel {
    fn from(c: GuildForum) -> Self {
        Self::GuildForum(c)
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum TagEmoji {
    Unicode {
        emoji_name: String,
    },
    Custom {
        emoji_id: EmojiId,
    },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Tag {
    /// the id of the tag
    pub id: TagId,
    /// the name of the tag (0-20 characters)
    pub name: String,
    /// whether this tag can only be added to or removed from threads by a member with the MANAGE_THREADS permission
    pub moderated: bool,
    #[serde(flatten)]
    default_reaction: TagEmoji,
}

serde_repr! {
    pub enum SortOrder: u8 {
        /// Sort forum posts by activity
		LatestActivity = 0,
        /// Sort forum posts by creation time (from most recent to oldest)
		CreationDate = 1,
    }
}

pub trait ChannelMarkup: Id<Id=ChannelId> {
    fn mention(&self) -> String {
        format!("<#{}>", self.id())
    }
}

impl<I: Id<Id=ChannelId>> ChannelMarkup for I {}

// #[derive(Serialize, Deserialize, Debug, Clone)]
// // #[serde(try_from = "RawOverwrite")]
// pub struct Overwrite {
//     /// role or user id
//     // #[serde(flatten)]
//     pub rm_id: OverwriteType,
//     /// permission bit set
//     pub allow: Permissions,
//     /// permission bit set
//     pub deny: Permissions,
// }

serde_num_tag! {
    /// See [permissions](https://discord.com/developers/docs/topics/permissions#permissions)
    /// for more information about the `allow` and `deny` fields.
    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub enum Overwrite = "type": u8 as OverwriteType {
        (0) = Role {
            id: RoleId,
            allow: Permissions,
            deny: Permissions,
        },
        (1) = Member {
            id: UserId,
            allow: Permissions,
            deny: Permissions,
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Copy)]
pub struct FollowedChannel {
    /// source channel id
    pub channel_id: ChannelId,
    /// created target webhook id
    pub webhook_id: WebhookId,
}

#[cfg(test)]
mod channel_tests {
    use super::*;

    fn assert(json: &str) {
        let channel: Channel = serde_json::from_str(json).unwrap();
        let back = serde_json::to_string_pretty(&channel).unwrap();
        assert_eq!(json, back)
    }

    #[test]
    fn overwrite() {
        let overwrite: Overwrite = serde_json::from_str(r#"{
  "id": "124",
  "type": 0,
  "allow": "10011",
  "deny": "100000"
}"#).unwrap();
        println!("overwrite = {:?}", overwrite);
    }

    #[test]
    fn guild_text() {
        assert(r#"{
  "type": 0,
  "id": "41771983423143937",
  "guild_id": "41771983423143937",
  "name": "general",
  "position": 6,
  "permission_overwrites": [],
  "rate_limit_per_user": 2,
  "nsfw": true,
  "topic": "24/7 chat about how to gank Mike #2",
  "last_message_id": "155117677105512449",
  "parent_id": "399942396007890945",
  "default_auto_archive_duration": 60
}"#)
    }

    #[test]
    fn guild_announcement() {
        assert(r#"{
  "type": 5,
  "id": "41771983423143937",
  "guild_id": "41771983423143937",
  "name": "important-news",
  "position": 6,
  "permission_overwrites": [],
  "nsfw": true,
  "topic": "Rumors about Half Life 3",
  "last_message_id": "155117677105512449",
  "parent_id": "399942396007890945",
  "default_auto_archive_duration": 60
}"#)
    }

    #[test]
    fn guild_voice() {
        assert(r#"{
  "type": 2,
  "id": "155101607195836416",
  "guild_id": "41771983423143937",
  "position": 5,
  "name": "ROCKET CHEESE",
  "parent_id": null,
  "bitrate": 64000,
  "last_message_id": "174629835082649376",
  "user_limit": 0,
  "rtc_region": null,
  "permission_overwrites": [],
  "rate_limit_per_user": 0,
  "nsfw": false
}"#)
    }

    #[test]
    fn dm() {
        assert(r#"{
  "type": 1,
  "id": "319674150115610528",
  "last_message_id": "3343820033257021450",
  "recipients": [
    {
      "id": "82198898841029460",
      "username": "test",
      "discriminator": "9999",
      "avatar": "33ecab261d4681afa4d85a04691c4a01"
    }
  ]
}"#)
    }

    #[test]
    fn group_dm() {
        assert(r#"{
  "type": 3,
  "id": "319674150115710528",
  "name": "Some test channel",
  "icon": null,
  "recipients": [
    {
      "id": "82198898841029460",
      "username": "test",
      "discriminator": "9999",
      "avatar": "33ecab261d4681afa4d85a04691c4a01"
    },
    {
      "id": "82198810841029460",
      "username": "test2",
      "discriminator": "9999",
      "avatar": "33ecab261d4681afa4d85a10691c4a01"
    }
  ],
  "last_message_id": "3343820033257021450",
  "owner_id": "82198810841029460"
}"#)
    }

    #[test]
    fn category_channel() {
        assert(r#"{
  "type": 4,
  "id": "399942396007890945",
  "guild_id": "290926798629997250",
  "name": "Test",
  "permission_overwrites": [],
  "nsfw": false,
  "position": 0
}"#)
    }

    #[test]
    fn thread() {
        let json = r#"{
  "type": 11,
  "id": "41771983423143937",
  "guild_id": "41771983423143937",
  "parent_id": "41771983423143937",
  "owner_id": "41771983423143937",
  "name": "don't buy dota-2",
  "last_message_id": "155117677105512449",
  "message_count": 1,
  "member_count": 5,
  "rate_limit_per_user": 2,
  "thread_metadata": {
    "archived": false,
    "auto_archive_duration": 1440,
    "archive_timestamp": "2021-04-12T23:40:39.855793Z",
    "locked": false
  },
  "total_message_sent": 1
}"#;
        let channel: Channel = serde_json::from_str(json).unwrap();
        let back = serde_json::to_string_pretty(&channel).unwrap();
        assert_eq!(json, back)
    }
}
use std::collections::HashMap;
use std::fmt::Formatter;

use itertools::{Either, Itertools};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{Error, Visitor};

use crate::cache::Cache;
use crate::model::channel::{Channel, Overwrite};
use crate::model::guild::GuildMember;
use crate::model::ids::*;
pub use crate::model::ids::RoleId;
bitflags! {
    pub struct Permissions: u64 {
		/// Allows creation of instant invites
        ///
        /// T, V, S
		const CREATE_INSTANT_INVITE = 1 << 0;
		/// Allows kicking members
		const KICK_MEMBERS = 1 << 1;
		/// Allows banning members
		const BAN_MEMBERS = 1 << 2;
		/// Allows all permissions and bypasses channel permission overwrites
		const ADMINISTRATOR = 1 << 3;
		/// Allows management and editing of channels
        ///
        /// T, V, S
		const MANAGE_CHANNELS = 1 << 4;
		/// Allows management and editing of the guild
		const MANAGE_GUILD = 1 << 5;
		/// Allows for the addition of reactions to messages
        ///
        /// T
		const ADD_REACTIONS = 1 << 6;
		/// Allows for viewing of audit logs
		const VIEW_AUDIT_LOG = 1 << 7;
		/// Allows for using priority speaker in a voice channel
        ///
        /// V
		const PRIORITY_SPEAKER = 1 << 8;
		/// Allows the user to go live
        ///
        /// V
		const STREAM = 1 << 9;
		/// Allows guild members to view a channel, which includes reading messages in text channels
        ///
        /// T, V, S
		const VIEW_CHANNEL = 1 << 10;
		/// Allows for sending messages in a channel
        ///
        /// T
		const SEND_MESSAGES = 1 << 11;
		/// Allows for sending of /tts messages
        ///
        /// T
		const SEND_TTS_MESSAGES = 1 << 12;
		/// Allows for deletion of other users messages
        ///
        /// T
		const MANAGE_MESSAGES = 1 << 13;
		/// Links sent by users with this permission will be auto-embedded
        ///
        /// T
		const EMBED_LINKS = 1 << 14;
		/// Allows for uploading images and files
        ///
        /// T
		const ATTACH_FILES = 1 << 15;
		/// Allows for reading of message history
        ///
        /// T
		const READ_MESSAGE_HISTORY = 1 << 16;
		/// Allows for using the @everyone tag to notify all users in a channel, and the `@here` tag
        /// to notify all online users in a channel
        ///
        /// T
		const MENTION_EVERYONE = 1 << 17;
		/// Allows the usage of custom emojis from other servers
        ///
        /// T
		const USE_EXTERNAL_EMOJIS = 1 << 18;
		/// Allows for viewing guild insights
		const VIEW_GUILD_INSIGHTS = 1 << 19;
		/// Allows for joining of a voice channel
        ///
        /// V, S
		const CONNECT = 1 << 20;
		/// Allows for speaking in a voice channel
        ///
        /// V
		const SPEAK = 1 << 21;
		/// Allows for muting members in a voice channel
        ///
        /// V, S
		const MUTE_MEMBERS = 1 << 22;
		/// Allows for deafening of members in a voice channel
        ///
        /// V, S
		const DEAFEN_MEMBERS = 1 << 23;
		/// Allows for moving of members between voice channels
        ///
        /// V, S
		const MOVE_MEMBERS = 1 << 24;
		/// Allows for using voice-activity-detection in a voice channel
        ///
        /// V, S
		const USE_VAD = 1 << 25;
		/// Allows for modification of own nickname
		const CHANGE_NICKNAME = 1 << 26;
		/// Allows for modification of other users nicknames
		const MANAGE_NICKNAMES = 1 << 27;
		/// Allows management and editing of roles
        ///
        /// T, V, S
		const MANAGE_ROLES = 1 << 28;
		/// Allows management and editing of webhooks
        ///
        /// T
		const MANAGE_WEBHOOKS = 1 << 29;
		/// Allows management and editing of emojis and stickers
		const MANAGE_EMOJIS_AND_STICKERS = 1 << 30;
		/// Allows members to use application commands, including slash commands and context menu
        /// commands.
        ///
        /// T
		const USE_APPLICATION_COMMANDS = 1 << 31;
		/// Allows for requesting to speak in stage channels. (This permission is under active
        /// development and may be changed or removed.)
        ///
        /// S
		const REQUEST_TO_SPEAK = 1 << 32;
        /// Allows for creating, editing, and deleting scheduled events
        ///
        /// V, S
        const MANAGE_EVENTS	= 1 << 33;
        /// Allows for deleting and archiving threads, and viewing all private threads
        ///
        /// T
        const MANAGE_THREADS = 1 << 34;
        /// Allows for creating and participating in threads
        ///
        /// T
        const USE_PUBLIC_THREADS = 1 << 35;
        /// Allows for creating and participating in private threads
        ///
        /// T
        const USE_PRIVATE_THREADS = 1 << 36;
        /// Allows the usage of custom stickers from other servers
        ///
        /// T
        const USE_EXTERNAL_STICKERS = 1 << 37;
        /// Allows for sending messages in threads
        ///
        /// T
        const SEND_MESSAGES_IN_THREADS = 1 << 38;
        /// Allows for launching activities (applications with the EMBEDDED flag) in a voice channel
        ///
        /// T
        const START_EMBEDDED_ACTIVITIES = 1 << 39;
        /// Allows for timing out users to prevent them from sending or reacting to messages in chat
        /// and threads, and from speaking in voice and stage channels
        const MODERATE_MEMBERS = 1 << 40;
        /// Allows for viewing role subscription insights
        const VIEW_CREATOR_MONETIZATION_ANALYTICS = 1 << 41;
        /// Allows for using soundboard in a voice channel
        ///
        /// V
        const USE_SOUNDBOARD = 1 << 42;
        /// Undocumented
        const UNKNOWN_43 = 1 << 43;
        /// Undocumented
        const UNKNOWN_44 = 1 << 44;
        /// Allows the usage of custom soundboard sounds from other servers
        ///
        /// V
        const USE_EXTERNAL_SOUNDS = 1 << 45;
        /// Allows sending voice messages
        /// T, V, S
        const SEND_VOICE_MESSAGES = 1 << 46;
        /// Undocumented
        const UNKNOWN_47 = 1 << 47;
        /// Undocumented, observed on [`Role`](Role)
        const UNKNOWN_48 = 1 << 48;
    }
}

use crate::model::message::Color;

impl Permissions {
    pub async fn get(cache: &Cache, member: &GuildMember, channel: &Channel, guild: GuildId) -> Self {
        let everyone = cache.everyone_role(&guild).await;

        let base_permissions = Self::base_permissions(
            cache, member, guild, &everyone,
        ).await;
        // let overwrites = cache.channel(channel).await;
        base_permissions.overwrites(member, channel.overwrites(), &everyone)
    }

    pub async fn get_own(cache: &Cache, channel: &Channel, guild: GuildId) -> Self {
        let member = cache.member(guild, cache.own_user().await).await.unwrap();
        Self::get(cache, &member, channel, guild).await
    }

    async fn base_permissions(cache: &Cache, member: &GuildMember, guild: GuildId, everyone: &Role) -> Self {
        let guild = cache.guild(guild).await.unwrap();
        if guild.owner { return Self::all(); }

        let permissions = member.roles.iter()
            .filter_map(|role| guild.roles.get(role))
            .fold(everyone.permissions, |perms, role_perms| perms | role_perms.permissions);
        if permissions.contains(Self::ADMINISTRATOR) {
            Self::all()
        } else {
            permissions
        }
    }

    fn overwrites(self, member: &GuildMember, overwrites: Option<&[Overwrite]>, everyone: &Role) -> Self {
        // ADMINISTRATOR overrides any potential permission overwrites, so there is nothing to do here.
        if self.contains(Self::ADMINISTRATOR) { return Self::all(); }

        let mut perms = self;

        if let Some(overwrites) = overwrites {
            let (role_overwrites, member_overwrites): (HashMap<_, _>, HashMap<_, _>) = overwrites.iter()
                .partition_map(|overwrite| match *overwrite {
                    Overwrite::Role { id, allow, deny } => Either::Left((id, (allow, deny))),
                    Overwrite::Member { id, allow, deny } => Either::Right((id, (allow, deny))),
                });

            // Find `@everyone` role overwrite and apply it.
            if let Some(&(allow, deny)) = role_overwrites.get(&everyone.id) {
                perms &= !deny;
                perms |= allow;
            }

            // Apply role specific overwrites.
            let (allow, deny) = member.roles.iter()
                .filter_map(|id| role_overwrites.get(id))
                .fold(
                    (Self::empty(), Self::empty()),
                    |(allow, deny), &(overwrite_allow, overwrite_deny)| (allow | overwrite_allow, deny | overwrite_deny),
                );
            perms &= !deny;
            perms |= allow;

            // Apply member specific overwrite.
            if let Some(&(allow, deny)) = member_overwrites.get(&member.id()) {
                perms &= !deny;
                perms |= allow;
            }
        }

        perms
    }
}

// can't just use `serde_bitflag!` because the bitflags are received as strings
impl<'de> Deserialize<'de> for Permissions {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct PermsVisitor;
        impl PermsVisitor {
            fn get_perms<E: Error>(s: &str) -> Result<Permissions, E> {
                let bits = s.parse()
                    .map_err(|e| E::custom(format!("Unable to parse bits as u64: {e}")))?;
                Permissions::from_bits(bits)
                    .ok_or_else(|| E::custom(format!("Unexpected `Permissions` bitflag value {bits}")))
            }
        }
        impl<'de> Visitor<'de> for PermsVisitor {
            type Value = Permissions;

            fn expecting(&self, f: &mut Formatter) -> std::fmt::Result {
                f.write_str("string (permissions)")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E> where E: Error {
                Self::get_perms(v)
            }

            fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E> where E: Error {
                Self::get_perms(v)
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E> where E: Error {
                Self::get_perms(&v)
            }
        }

        d.deserialize_str(PermsVisitor)
    }
}

impl Serialize for Permissions {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        self.bits.serialize(s)
    }
}

/// Roles represent a set of permissions attached to a group of users. Roles have unique names,
/// colors, and can be "pinned" to the side bar, causing their members to be listed separately.
/// Roles are unique per guild, and can have separate permission profiles for the global context
/// (guild) and channel context. The `@everyone` role has the same ID as the guild it belongs to.
/// Roles without colors (`color == 0`) do not count towards the final computed color in the user
/// list.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Role {
    /// role id
    pub id: RoleId,
    /// role name
    pub name: String,
    /// integer representation of hexadecimal color code
    pub color: Color,
    /// if this role is pinned in the user listing
    pub hoist: bool,
    /// position of this role
    pub position: u32,
    /// permission bit set
    pub permissions: Permissions,
    /// whether this role is managed by an integration
    pub managed: bool,
    /// whether this role is mentionable
    pub mentionable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<RoleTags>,
}
id_impl!(Role => RoleId);

pub trait RoleMarkup: Id<Id=RoleId> {
    fn mention(&self) -> String {
        format!("<@&{}>", self.id())
    }
}

impl<I: Id<Id=RoleId>> RoleMarkup for I {}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct RoleTags {
    /// the id of the bot this role belongs to
    bot_id: Option<ApplicationId>,
    /// the id of the integration this role belongs to
    integration_id: Option<InteractionId>,
    // todo docs say the type of this is `null`... idk how to handle that lol. probably have to make
    //  custom visitor based deserializer
    /// whether this is the guild's premium subscriber role
    premium_subscriber: Option<()>,
}

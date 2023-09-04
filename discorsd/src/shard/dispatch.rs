use std::collections::hash_map::Entry;
use std::collections::HashSet;
use std::convert::{TryFrom, TryInto};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use serde_derive::{Deserialize, Serialize};

use crate::cache::{Cache, IdMap, Update};
use crate::model::auto_moderation::{Action, AutoModRule, TriggerType};
use crate::model::channel::{Channel, ChannelType, Thread, ThreadMember};
use crate::model::components::ActionRow;
use crate::model::emoji::{CustomEmoji, Emoji};
use crate::model::guild::{ExplicitFilterLevel, Guild, GuildFeature, GuildMember, Integration, MfaLevel, NotificationLevel, PremiumTier, SystemChannelFlags, UnavailableGuild, VerificationLevel};
use crate::model::ids::*;
use crate::model::interaction::{ApplicationCommandData, Interaction, InteractionData};
use crate::model::message::{Attachment, ChannelMention, ChannelMessageId, Embed, Message, MessageActivity, MessageApplication, MessageFlags, MessageInteraction, MessageReference, MessageType, Reaction, StickerItem};
use crate::model::permissions::{Permissions, Role};
use crate::model::user::User;
use crate::model::voice::VoiceState;
use crate::shard::model::{Activity, StatusType};

#[allow(clippy::large_enum_variant)]
#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "t", content = "d", rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum DispatchPayload {
    // Connection
    Ready(Ready),
    Resumed(Resumed),

    // Channels
    ChannelCreate(ChannelCreate),
    ChannelUpdate(ChannelUpdate),
    ChannelDelete(ChannelDelete),
    ChannelPinsUpdate(ChannelPinsUpdate),

    // Threads
    ThreadCreate(ThreadCreate),
    ThreadUpdate(ThreadUpdate),
    ThreadDelete(ThreadDelete),
    ThreadListSync(ThreadListSync),
    ThreadMemberUpdate(ThreadMemberUpdate),
    ThreadMembersUpdate(ThreadMembersUpdate),

    // Guilds
    GuildCreate(GuildCreate),
    GuildUpdate(GuildUpdate),
    GuildDelete(GuildDelete),
    GuildBanAdd(BanAdd),
    GuildBanRemove(BanRemove),
    GuildEmojisUpdate(EmojiUpdate),
    GuildStickersUpdate(StickerUpdate),
    GuildIntegrationsUpdate(GuildIntegrationsUpdate),
    GuildMemberAdd(GuildMemberAdd),
    GuildMemberRemove(GuildMemberRemove),
    GuildMemberUpdate(GuildMemberUpdate),
    GuildMembersChunk(GuildMembersChunk),
    GuildRoleCreate(GuildRoleCreate),
    GuildRoleUpdate(GuildRoleUpdate),
    GuildRoleDelete(GuildRoleDelete),
    GuildScheduledEventCreate(GuildScheduledEventCreate),
    GuildScheduledEventUpdate(GuildScheduledEventUpdate),
    GuildScheduledEventDelete(GuildScheduledEventDelete),
    GuildScheduledEventUserAdd(GuildScheduledEventUserAdd),
    GuildScheduledEventUserRemove(GuildScheduledEventUserRemove),

    // Integrations
    IntegrationCreate(IntegrationCreate),
    IntegrationUpdate(IntegrationUpdate),
    IntegrationDelete(IntegrationDelete),

    // Invites
    InviteCreate(InviteCreate),
    InviteDelete(InviteDelete),

    // Messages
    MessageCreate(MessageCreate),
    // clippy says to box this because its a big boi... meh?
    MessageUpdate(MessageUpdate),
    MessageDelete(MessageDelete),
    MessageDeleteBulk(MessageDeleteBulk),
    MessageReactionAdd(ReactionAdd),
    MessageReactionRemove(ReactionRemove),
    MessageReactionRemoveAll(ReactionRemoveAll),
    MessageReactionRemoveEmoji(ReactionRemoveEmoji),

    // Stage
    StageInstanceCreate(StageInstanceCreate),
    StageInstanceUpdate(StageInstanceUpdate),
    StageInstanceDelete(StageInstanceDelete),

    // Presence
    PresenceUpdate(PresenceUpdate),
    TypingStart(TypingStart),
    UserUpdate(UserUpdate),

    // Voice
    VoiceStateUpdate(VoiceStateUpdate),
    VoiceServerUpdate(VoiceServerUpdate),

    // Webhooks
    WebhooksUpdate(WebhookUpdate),

    // Commands
    InteractionCreate(InteractionCreate),
    // ApplicationCommandCreate(ApplicationCommandCreate),
    // ApplicationCommandUpdate(ApplicationCommandUpdate),
    // ApplicationCommandDelete(ApplicationCommandDelete),
    ApplicationCommandPermissionsUpdate(ApplicationCommandPermissionsUpdate),

    // Auto mod
    AutoModerationRuleCreate(AutoModerationRuleCreate),
    AutoModerationRuleUpdate(AutoModerationRuleUpdate),
    AutoModerationRuleDelete(AutoModerationRuleDelete),
    AutoModerationActionExecution(AutoModerationActionExecution),
}

#[async_trait]
impl Update for DispatchPayload {
    async fn update(&self, cache: &Cache) {
        use DispatchPayload::*;
        match self {
            Ready(ready) => ready.update(cache).await,
            Resumed(resumed) => resumed.update(cache).await,
            ChannelCreate(create) => create.update(cache).await,
            ChannelUpdate(update) => update.update(cache).await,
            ChannelDelete(delete) => delete.update(cache).await,
            ChannelPinsUpdate(pins_update) => pins_update.update(cache).await,
            GuildCreate(create) => create.update(cache).await,
            GuildUpdate(update) => update.update(cache).await,
            GuildDelete(delete) => delete.update(cache).await,
            GuildBanAdd(ban_add) => ban_add.update(cache).await,
            GuildBanRemove(ban_remove) => ban_remove.update(cache).await,
            GuildEmojisUpdate(emojis_update) => emojis_update.update(cache).await,
            GuildIntegrationsUpdate(integrations) => integrations.update(cache).await,
            IntegrationUpdate(update) => update.update(cache).await,
            GuildMemberAdd(member_add) => member_add.update(cache).await,
            GuildMemberRemove(member_remove) => member_remove.update(cache).await,
            GuildMemberUpdate(member_update) => member_update.update(cache).await,
            GuildMembersChunk(members_chunk) => members_chunk.update(cache).await,
            GuildRoleCreate(role_create) => role_create.update(cache).await,
            GuildRoleUpdate(role_update) => role_update.update(cache).await,
            GuildRoleDelete(role_delete) => role_delete.update(cache).await,
            InviteCreate(invite_create) => invite_create.update(cache).await,
            InviteDelete(invite_delete) => invite_delete.update(cache).await,
            MessageCreate(message_create) => message_create.update(cache).await,
            MessageUpdate(message_update) => message_update.update(cache).await,
            MessageDelete(message_delete) => message_delete.update(cache).await,
            MessageDeleteBulk(message_delete_bulk) => message_delete_bulk.update(cache).await,
            MessageReactionAdd(message_reaction_add) => message_reaction_add.update(cache).await,
            MessageReactionRemove(message_reaction_remove) => message_reaction_remove.update(cache).await,
            MessageReactionRemoveAll(message_reaction_remove_all) => message_reaction_remove_all.update(cache).await,
            MessageReactionRemoveEmoji(message_reaction_remove_emoji) => message_reaction_remove_emoji.update(cache).await,
            PresenceUpdate(presence_update) => presence_update.update(cache).await,
            TypingStart(typing_start) => typing_start.update(cache).await,
            UserUpdate(user_update) => user_update.update(cache).await,
            VoiceStateUpdate(voice_state_update) => voice_state_update.update(cache).await,
            VoiceServerUpdate(voice_server_update) => voice_server_update.update(cache).await,
            WebhooksUpdate(webhooks_update) => webhooks_update.update(cache).await,
            InteractionCreate(interactions) => interactions.update(cache).await,
            // ApplicationCommandCreate(create) => create.update(cache).await,
            // ApplicationCommandUpdate(update) => update.update(cache).await,
            // ApplicationCommandDelete(delete) => delete.update(cache).await,
            ApplicationCommandPermissionsUpdate(update) => update.update(cache).await,
            // todo
            ThreadCreate(_) => {}
            ThreadUpdate(_) => {}
            ThreadDelete(_) => {}
            ThreadListSync(_) => {}
            ThreadMemberUpdate(_) => {}
            ThreadMembersUpdate(_) => {}
            GuildStickersUpdate(_) => {}
            GuildScheduledEventCreate(_) => {}
            GuildScheduledEventUpdate(_) => {}
            GuildScheduledEventDelete(_) => {}
            GuildScheduledEventUserAdd(_) => {}
            GuildScheduledEventUserRemove(_) => {}
            IntegrationCreate(_) => {}
            IntegrationDelete(_) => {}
            StageInstanceCreate(_) => {}
            StageInstanceUpdate(_) => {}
            StageInstanceDelete(_) => {}
            AutoModerationRuleCreate(_) => {}
            AutoModerationRuleUpdate(_) => {}
            AutoModerationRuleDelete(_) => {}
            AutoModerationActionExecution(_) => {}
        };
    }
}

// Connection events

/// The ready event is dispatched when a client has completed the initial handshake with the gateway
/// (for new sessions). The ready event can be the largest and most complex event the gateway will
/// send, as it contains all the state required for a client to begin interacting with the rest of
/// the platform.
///
/// `guilds` are the guilds of which your bot is a member. They start out as unavailable when you
/// connect to the gateway. As they become available, your bot will be notified via
/// [`GuildCreate`](crate::shard::dispatch::GuildCreate) events. `private_channels` will be an empty
/// array. As bots receive private messages, they will be notified via
/// [`ChannelCreate`](crate::shard::dispatch::ChannelCreate) events.
#[derive(Deserialize, Debug, Clone)]
#[non_exhaustive]
pub struct Ready {
    /// gateway version
    pub v: u8,
    /// information about the user including email
    pub user: User,
    /// the guilds the user is in
    pub guilds: Vec<UnavailableGuild>,
    /// used for resuming connections
    pub session_id: String,
    /// Gateway URL for resuming connections
    pub resume_gateway_url: String,
    /// the shard information associated with this session, if sent when identifying
    pub shard: Option<(u64, u64)>,
    /// partial application information
    pub application: PartialApplication,
}

/// Partial information about a Bot's application containing it's id and flags.
#[derive(Copy, Clone, Deserialize, Debug)]
pub struct PartialApplication {
    /// the id of the app
    pub id: ApplicationId,
    /// the application's public flags
    pub flags: Option<u32>,
}

bitflags! {
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
    pub struct ApplicationFlags: u32 {
        const GATEWAY_PRESENCE = 1 << 12;
        const GATEWAY_PRESENCE_LIMITED = 1 << 13;
        const GATEWAY_GUILD_MEMBERS = 1 << 14;
        const GATEWAY_GUILD_MEMBERS_LIMITED = 1 << 15;
        const VERIFICATION_PENDING_GUILD_LIMIT = 1 << 16;
        const EMBEDDED = 1 << 17;
    }
}
serde_bitflag!(ApplicationFlags: u32);

#[async_trait]
impl Update for Ready {
    async fn update(&self, cache: &Cache) {
        let _res = cache.application.set(self.application);
        // *cache.application.write().await = Some(self.application);
        *cache.user.write().await = Some(self.user.clone());
        cache.users.write().await.insert(self.user.clone());
        cache.unavailable_guilds.write().await.extend(self.guilds.clone());
    }
}

/// The resumed event is dispatched when a client has sent a resume payload to the gateway
/// (for resuming existing sessions).
#[derive(Deserialize, Debug, Clone)]
pub struct Resumed {
    _trace: Vec<serde_json::Value>,
}

#[async_trait]
impl Update for Resumed {
    async fn update(&self, _cache: &Cache) {
        println!("self._trace = {:?}", self._trace);
        // don't think we need to update anything here
    }
}

// Channel Events

#[derive(Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct ChannelCreate {
    pub(crate) channel: Channel,
}

#[async_trait]
impl Update for ChannelCreate {
    async fn update(&self, cache: &Cache) {
        let channel = &self.channel;
        if let Some(guild) = channel.guild_id() {
            cache.guilds.write().await
                .entry(guild)
                .and_modify(|guild| guild.channels.insert(channel.clone()));
        }
        cache.channel_types.write().await.insert(channel.id(), channel.variant_type());
        match channel {
            Channel::Text(text) => {
                cache.channels.write().await.insert(text.clone());
            }
            Channel::Dm(dm) => {
                let (by_user, by_id) = &mut *cache.dms.write().await;
                by_user.insert(dm.recipient.id, dm.id);
                by_id.insert(dm.clone());
            }
            Channel::Voice(_) => {
                // voice not implemented yet (ever)
            }
            Channel::GroupDm(_) => unreachable!("Bots cannot be in GroupDm channels"),
            Channel::Category(category) => {
                cache.categories.write().await.insert(category.clone());
            }
            Channel::Announcement(news) => {
                cache.news.write().await.insert(news.clone());
            }
            // Channel::Store(store) => {
            //     cache.stores.write().await.insert(store.clone())
            // }
            // todo
            Channel::AnnouncementThread(_) => {}
            Channel::PublicThread(_) => {}
            Channel::PrivateThread(_) => {}
            Channel::GuildStageVoice(_) => {}
            Channel::GuildDirectory(_) => {}
            Channel::GuildForum(_) => {}
        };
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct ChannelUpdate {
    channel: Channel,
}

#[async_trait]
impl Update for ChannelUpdate {
    async fn update(&self, cache: &Cache) {
        let channel = &self.channel;
        if let Some(guild) = channel.guild_id() {
            cache.guilds.write().await
                .entry(guild)
                .and_modify(|guild| {
                    guild.channels.entry(channel.id())
                        .and_modify(|old| *old = channel.clone());
                });
        }
        match channel {
            Channel::Text(channel) => {
                if let Some(text) = cache.channels.write().await.get_mut(&channel) {
                    *text = channel.clone();
                }
            }
            Channel::Dm(channel) => {
                let (_, by_channel) = &mut *cache.dms.write().await;
                if let Some(dm) = by_channel.get_mut(&channel) {
                    *dm = channel.clone();
                }
            }
            Channel::Voice(_) => {
                // voice not implemented yet (ever)
            }
            Channel::GroupDm(_) => unreachable!("Bots cannot be in GroupDm channels"),
            Channel::Category(channel) => {
                if let Some(category) = cache.categories.write().await.get_mut(&channel) {
                    *category = channel.clone();
                }
            }
            Channel::Announcement(channel) => {
                if let Some(news) = cache.news.write().await.get_mut(&channel) {
                    *news = channel.clone();
                }
            }
            // Channel::Store(channel) => {
            //     if let Some(store) = cache.stores.write().await.get_mut(&channel) {
            //         *store = channel.clone();
            //     }
            // }
            // todo
            Channel::AnnouncementThread(_) => {}
            Channel::PublicThread(_) => {}
            Channel::PrivateThread(_) => {}
            Channel::GuildStageVoice(_) => {}
            Channel::GuildDirectory(_) => {}
            Channel::GuildForum(_) => {}
        };
    }
}

/// Sent when a channel relevant to the current user is deleted.
#[derive(Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct ChannelDelete {
    channel: Channel,
}

#[async_trait]
impl Update for ChannelDelete {
    async fn update(&self, cache: &Cache) {
        println!("DELETE channel = {self:#?}");
        cache.channel_types.write().await.remove(&self.channel.id());
        match &self.channel {
            Channel::Text(text) => { cache.channels.write().await.remove(text); }
            Channel::Dm(dm) => {
                let (by_user, by_channel) = &mut *cache.dms.write().await;
                by_user.remove(&dm.recipient.id);
                by_channel.remove(dm);
            }
            Channel::Category(cat) => { cache.categories.write().await.remove(cat); }
            Channel::Announcement(news) => { cache.news.write().await.remove(news); }
            // Channel::Store(store) => { cache.stores.write().await.remove(store); },
            Channel::Voice(_) | Channel::GroupDm(_) => {}
            // todo
            Channel::AnnouncementThread(_) => {}
            Channel::PublicThread(_) => {}
            Channel::PrivateThread(_) => {}
            Channel::GuildStageVoice(_) => {}
            Channel::GuildDirectory(_) => {}
            Channel::GuildForum(_) => {}
        };
    }
}

#[derive(Deserialize, Debug, Clone, Copy)]
pub struct ChannelPinsUpdate {
    /// the id of the guild
    pub guild_id: Option<GuildId>,
    /// the id of the channel
    pub channel_id: ChannelId,
    /// the time at which the most recent pinned message was pinned
    pub last_pin_timestamp: Option<DateTime<Utc>>,
}

#[async_trait]
impl Update for ChannelPinsUpdate {
    async fn update(&self, cache: &Cache) {
        let Self { guild_id, channel_id, last_pin_timestamp } = &self;
        let last_pin_timestamp = *last_pin_timestamp;
        // don't forget that the lock on `channel_types` is held throughout all branches
        #[allow(clippy::significant_drop_in_scrutinee)]
        match cache.channel_types.read().await.get(channel_id) {
            Some(ChannelType::Text) => {
                cache.channels.write().await.entry(&channel_id)
                    .and_modify(|channel| {
                        channel.last_pin_timestamp = last_pin_timestamp;
                    });
            }
            Some(ChannelType::Dm) => {
                cache.dms.write().await.1.entry(&channel_id)
                    .and_modify(|channel| {
                        channel.last_pin_timestamp = last_pin_timestamp;
                    });
            }
            Some(ChannelType::Announcement) => {
                cache.news.write().await.entry(&channel_id)
                    .and_modify(|channel| {
                        channel.last_pin_timestamp = last_pin_timestamp;
                    });
            }
            Some(ChannelType::Voice | ChannelType::Category) => {}
            Some(ChannelType::GroupDm) | None => {}
            // todo
            Some(ChannelType::AnnouncementThread) => {}
            Some(ChannelType::PublicThread) => {}
            Some(ChannelType::PrivateThread) => {}
            Some(ChannelType::GuildStageVoice) => {}
            Some(ChannelType::GuildDirectory) => {}
            Some(ChannelType::GuildForum) => {}
        }
        if let Some(guild_id) = guild_id {
            cache.guilds.write().await.entry(guild_id)
                .and_modify(|guild| {
                    guild.channels.entry(&channel_id)
                        .and_modify(|channel| match channel {
                            Channel::Text(channel) => channel.last_pin_timestamp = last_pin_timestamp,
                            Channel::Announcement(channel) => channel.last_pin_timestamp = last_pin_timestamp,
                            // no last timestamp
                            Channel::Voice(_) | Channel::Category(_) => {}
                            // not in a guild
                            Channel::Dm(_) | Channel::GroupDm(_) => {}
                            // todo
                            Channel::AnnouncementThread(_) => {}
                            Channel::PublicThread(_) => {}
                            Channel::PrivateThread(_) => {}
                            Channel::GuildStageVoice(_) => {}
                            Channel::GuildDirectory(_) => {}
                            Channel::GuildForum(_) => {}
                        });
                });
        }
    }
}

// Guild Events

/// This event can be sent in three different scenarios:
/// 1. When a user is initially connecting, to lazily load and backfill information for all
/// unavailable guilds sent in the [Ready] event. Guilds that are unavailable due to an outage will
/// send a [`GuildDelete`] event.
/// 2. When a [Guild] becomes available again to the client.
/// 3. When the current user joins a new Guild.
/// The inner payload is a [Guild], with all the extra fields specified.
#[derive(Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct GuildCreate {
    pub(crate) guild: Guild,
    // todo this is in the guild
    // pub(crate) threads: Vec<Thread>,
}

#[test]
fn test_guild_create() {
    // const JSON: &str = r#"{"t":"GUILD_CREATE","s":2,"op":0,"d":{"owner_id":"243418816510558208","system_channel_id":"492122906864779276","guild_hashes":{"version":1,"roles":{"omitted":false,"hash":"OROePw"},"metadata":{"omitted":false,"hash":"aD+z8Q"},"channels":{"omitted":false,"hash":"AiVDlg"}},"system_channel_flags":0,"guild_scheduled_events":[],"afk_timeout":300,"region":"deprecated","stickers":[],"rules_channel_id":null,"stage_instances":[],"members":[{"user":{"username":"Avalon Bot (DEV)","public_flags":0,"id":"780237314734686208","global_name":null,"display_name":null,"discriminator":"6988","bot":true,"avatar_decoration":null,"avatar":"5b4212af734408382daff0cb4c0be97d"},"roles":["780239802196426773"],"premium_since":null,"pending":false,"nick":null,"mute":false,"joined_at":"2020-11-23T01:14:25.047000+00:00","flags":0,"deaf":false,"communication_disabled_until":null,"avatar":null}],"premium_subscription_count":0,"application_id":null,"safety_alerts_channel_id":null,"large":false,"default_message_notifications":0,"inventory_settings":null,"mfa_level":0,"discovery_splash":null,"max_video_channel_users":25,"description":null,"explicit_content_filter":0,"nsfw_level":0,"afk_channel_id":null,"icon":null,"id":"492122906864779274","hub_type":null,"incidents_data":null,"public_updates_channel_id":null,"voice_states":[],"home_header":null,"member_count":8,"vanity_url_code":null,"preferred_locale":"en-US","presences":[],"embedded_activities":[],"splash":null,"features":[],"application_command_counts":{"1":38},"threads":[],"banner":null,"joined_at":"2020-11-23T01:14:25.047000+00:00","max_members":500000,"roles":[{"version":0,"unicode_emoji":null,"tags":{},"position":0,"permissions":"137411140509249","name":"@everyone","mentionable":false,"managed":false,"id":"492122906864779274","icon":null,"hoist":false,"flags":0,"color":0},{"version":0,"unicode_emoji":null,"tags":{"bot_id":"492123129573802004"},"position":3,"permissions":"6509947968","name":"Avalon Bot","mentionable":false,"managed":true,"id":"492123963728855044","icon":null,"hoist":false,"flags":0,"color":0},{"version":0,"unicode_emoji":null,"tags":{},"position":6,"permissions":"6546775617","name":"myrole","mentionable":true,"managed":false,"id":"592892380609511445","icon":null,"hoist":false,"flags":0,"color":15844367},{"version":0,"unicode_emoji":null,"tags":{"bot_id":"713764953525583897"},"position":5,"permissions":"6442576960","name":"Rutgers Course Sniper","mentionable":false,"managed":true,"id":"713792310659514462","icon":null,"hoist":false,"flags":0,"color":0},{"version":1688673986671,"unicode_emoji":null,"tags":{"bot_id":"780237314734686208"},"position":2,"permissions":"8","name":"Avalon Bot (DEV)","mentionable":false,"managed":true,"id":"780239802196426773","icon":null,"hoist":false,"flags":0,"color":0},{"version":0,"unicode_emoji":null,"tags":{},"position":1,"permissions":"6546775617","name":"\"pretty color\"","mentionable":false,"managed":false,"id":"788583747771433020","icon":null,"hoist":false,"flags":0,"color":5533306},{"version":0,"unicode_emoji":null,"tags":{},"position":1,"permissions":"6546775617","name":"new role","mentionable":false,"managed":false,"id":"788893153209745409","icon":null,"hoist":false,"flags":0,"color":0},{"version":0,"unicode_emoji":null,"tags":{"bot_id":"928797878669955082"},"position":1,"permissions":"8799381367872","name":"Counter","mentionable":false,"managed":true,"id":"928808168023281716","icon":null,"hoist":false,"flags":0,"color":0},{"version":0,"unicode_emoji":null,"tags":{"bot_id":"268547439714238465"},"position":1,"permissions":"274878221376","name":"Scryfall","mentionable":false,"managed":true,"id":"991036300272480349","icon":null,"hoist":false,"flags":0,"color":0}],"emojis":[{"version":0,"roles":[],"require_colons":true,"name":"echan","managed":false,"id":"786354378885824513","available":true,"animated":false},{"version":0,"roles":[],"require_colons":true,"name":"bruh","managed":false,"id":"788901233016045599","available":true,"animated":false},{"version":0,"roles":[],"require_colons":true,"name":"Bonk","managed":false,"id":"839001103391260692","available":true,"animated":false},{"version":0,"roles":[],"require_colons":true,"name":"Bonk_2_Electric_Boogaloo","managed":false,"id":"839001103844114432","available":true,"animated":false},{"version":0,"roles":[],"require_colons":true,"name":"Bonk_cropped","managed":false,"id":"839001439601950750","available":true,"animated":false},{"version":0,"roles":[],"require_colons":true,"name":"Bonk_2_Electric_Boogaloo_cropped","managed":false,"id":"839001744443703317","available":true,"animated":false}],"latest_onboarding_question_id":null,"verification_level":0,"lazy":true,"premium_progress_bar_enabled":false,"nsfw":false,"unavailable":false,"channels":[{"version":0,"type":4,"position":0,"permission_overwrites":[],"name":"Text Channels","id":"492122906864779275","flags":0},{"version":0,"type":0,"topic":null,"rate_limit_per_user":0,"position":0,"permission_overwrites":[{"type":0,"id":"492122906864779274","deny":"0","allow":"0"}],"parent_id":"492122906864779275","name":"general","last_message_id":"991036430912454696","id":"492122906864779276","flags":0},{"version":0,"type":4,"position":2,"permission_overwrites":[],"name":"Voice Channels","id":"492122907300855818","flags":0},{"version":0,"user_limit":0,"type":2,"rtc_region":null,"rate_limit_per_user":0,"position":0,"permission_overwrites":[],"parent_id":"492122907300855818","name":"General","last_message_id":null,"id":"492122907300855819","flags":0,"bitrate":64000},{"version":0,"type":0,"topic":null,"rate_limit_per_user":0,"position":1,"permission_overwrites":[{"type":0,"id":"780239802196426773","deny":"1024","allow":"0"}],"parent_id":"492122906864779275","name":"avalon_bot","last_message_id":"849017251914842192","id":"492122962451759124","flags":0},{"version":1667238419348,"type":0,"topic":null,"rate_limit_per_user":0,"position":2,"permission_overwrites":[{"type":0,"id":"492123963728855044","deny":"1024","allow":"0"}],"parent_id":"492122906864779275","name":"dev_bot","last_pin_timestamp":"2023-07-06T20:05:55+00:00","last_message_id":"1131797440643735683","id":"780240796690808912","flags":0},{"version":0,"type":0,"topic":"topic lol","rate_limit_per_user":0,"position":3,"permission_overwrites":[{"type":1,"id":"592500196303437826","deny":"262144","allow":"16"}],"parent_id":"492122906864779275","name":"test-member-overwrite","last_message_id":"793351162396278814","id":"781369592982929408","flags":0}],"name":"Bots Bots Bots","max_stage_video_channel_users":50,"premium_tier":0}}"#;
    const JSON: &str = r#"{
  "owner_id": "243418816510558208",
  "system_channel_id": "492122906864779276",
  "guild_hashes": {
    "version": 1,
    "roles": {
      "omitted": false,
      "hash": "OROePw"
    },
    "metadata": {
      "omitted": false,
      "hash": "aD+z8Q"
    },
    "channels": {
      "omitted": false,
      "hash": "AiVDlg"
    }
  },
  "system_channel_flags": 0,
  "guild_scheduled_events": [],
  "afk_timeout": 300,
  "region": "deprecated",
  "stickers": [],
  "rules_channel_id": null,
  "stage_instances": [],
  "members": [
    {
      "user": {
        "username": "Avalon Bot (DEV)",
        "public_flags": 0,
        "id": "780237314734686208",
        "global_name": null,
        "display_name": null,
        "discriminator": "6988",
        "bot": true,
        "avatar_decoration": null,
        "avatar": "5b4212af734408382daff0cb4c0be97d"
      },
      "roles": [
        "780239802196426773"
      ],
      "premium_since": null,
      "pending": false,
      "nick": null,
      "mute": false,
      "joined_at": "2020-11-23T01:14:25.047000+00:00",
      "flags": 0,
      "deaf": false,
      "communication_disabled_until": null,
      "avatar": null
    }
  ],
  "premium_subscription_count": 0,
  "application_id": null,
  "safety_alerts_channel_id": null,
  "large": false,
  "default_message_notifications": 0,
  "inventory_settings": null,
  "mfa_level": 0,
  "discovery_splash": null,
  "max_video_channel_users": 25,
  "description": null,
  "explicit_content_filter": 0,
  "nsfw_level": 0,
  "afk_channel_id": null,
  "icon": null,
  "id": "492122906864779274",
  "hub_type": null,
  "incidents_data": null,
  "public_updates_channel_id": null,
  "voice_states": [],
  "home_header": null,
  "member_count": 8,
  "vanity_url_code": null,
  "preferred_locale": "en-US",
  "presences": [],
  "embedded_activities": [],
  "splash": null,
  "features": [],
  "application_command_counts": {
    "1": 38
  },
  "threads": [],
  "banner": null,
  "joined_at": "2020-11-23T01:14:25.047000+00:00",
  "max_members": 500000,
  "roles": [
    {
      "version": 0,
      "unicode_emoji": null,
      "tags": {},
      "position": 0,
      "permissions": "137411140509249",
      "name": "@everyone",
      "mentionable": false,
      "managed": false,
      "id": "492122906864779274",
      "icon": null,
      "hoist": false,
      "flags": 0,
      "color": 0
    },
    {
      "version": 0,
      "unicode_emoji": null,
      "tags": {
        "bot_id": "492123129573802004"
      },
      "position": 3,
      "permissions": "6509947968",
      "name": "Avalon Bot",
      "mentionable": false,
      "managed": true,
      "id": "492123963728855044",
      "icon": null,
      "hoist": false,
      "flags": 0,
      "color": 0
    },
    {
      "version": 0,
      "unicode_emoji": null,
      "tags": {},
      "position": 6,
      "permissions": "6546775617",
      "name": "myrole",
      "mentionable": true,
      "managed": false,
      "id": "592892380609511445",
      "icon": null,
      "hoist": false,
      "flags": 0,
      "color": 15844367
    },
    {
      "version": 0,
      "unicode_emoji": null,
      "tags": {
        "bot_id": "713764953525583897"
      },
      "position": 5,
      "permissions": "6442576960",
      "name": "Rutgers Course Sniper",
      "mentionable": false,
      "managed": true,
      "id": "713792310659514462",
      "icon": null,
      "hoist": false,
      "flags": 0,
      "color": 0
    },
    {
      "version": 1688673986671,
      "unicode_emoji": null,
      "tags": {
        "bot_id": "780237314734686208"
      },
      "position": 2,
      "permissions": "8",
      "name": "Avalon Bot (DEV)",
      "mentionable": false,
      "managed": true,
      "id": "780239802196426773",
      "icon": null,
      "hoist": false,
      "flags": 0,
      "color": 0
    },
    {
      "version": 0,
      "unicode_emoji": null,
      "tags": {},
      "position": 1,
      "permissions": "6546775617",
      "name": "\"pretty color\"",
      "mentionable": false,
      "managed": false,
      "id": "788583747771433020",
      "icon": null,
      "hoist": false,
      "flags": 0,
      "color": 5533306
    },
    {
      "version": 0,
      "unicode_emoji": null,
      "tags": {},
      "position": 1,
      "permissions": "6546775617",
      "name": "new role",
      "mentionable": false,
      "managed": false,
      "id": "788893153209745409",
      "icon": null,
      "hoist": false,
      "flags": 0,
      "color": 0
    },
    {
      "version": 0,
      "unicode_emoji": null,
      "tags": {
        "bot_id": "928797878669955082"
      },
      "position": 1,
      "permissions": "8799381367872",
      "name": "Counter",
      "mentionable": false,
      "managed": true,
      "id": "928808168023281716",
      "icon": null,
      "hoist": false,
      "flags": 0,
      "color": 0
    },
    {
      "version": 0,
      "unicode_emoji": null,
      "tags": {
        "bot_id": "268547439714238465"
      },
      "position": 1,
      "permissions": "274878221376",
      "name": "Scryfall",
      "mentionable": false,
      "managed": true,
      "id": "991036300272480349",
      "icon": null,
      "hoist": false,
      "flags": 0,
      "color": 0
    }
  ],
  "emojis": [
    {
      "version": 0,
      "roles": [],
      "require_colons": true,
      "name": "echan",
      "managed": false,
      "id": "786354378885824513",
      "available": true,
      "animated": false
    },
    {
      "version": 0,
      "roles": [],
      "require_colons": true,
      "name": "bruh",
      "managed": false,
      "id": "788901233016045599",
      "available": true,
      "animated": false
    },
    {
      "version": 0,
      "roles": [],
      "require_colons": true,
      "name": "Bonk",
      "managed": false,
      "id": "839001103391260692",
      "available": true,
      "animated": false
    },
    {
      "version": 0,
      "roles": [],
      "require_colons": true,
      "name": "Bonk_2_Electric_Boogaloo",
      "managed": false,
      "id": "839001103844114432",
      "available": true,
      "animated": false
    },
    {
      "version": 0,
      "roles": [],
      "require_colons": true,
      "name": "Bonk_cropped",
      "managed": false,
      "id": "839001439601950750",
      "available": true,
      "animated": false
    },
    {
      "version": 0,
      "roles": [],
      "require_colons": true,
      "name": "Bonk_2_Electric_Boogaloo_cropped",
      "managed": false,
      "id": "839001744443703317",
      "available": true,
      "animated": false
    }
  ],
  "latest_onboarding_question_id": null,
  "verification_level": 0,
  "lazy": true,
  "premium_progress_bar_enabled": false,
  "nsfw": false,
  "unavailable": false,
  "channels": [
    {
      "version": 0,
      "type": 4,
      "position": 0,
      "permission_overwrites": [],
      "name": "Text Channels",
      "id": "492122906864779275",
      "flags": 0
    },
    {
      "version": 0,
      "type": 0,
      "topic": null,
      "rate_limit_per_user": 0,
      "position": 0,
      "permission_overwrites": [
        {
          "type": 0,
          "id": "492122906864779274",
          "deny": "0",
          "allow": "0"
        }
      ],
      "parent_id": "492122906864779275",
      "name": "general",
      "last_message_id": "991036430912454696",
      "id": "492122906864779276",
      "flags": 0
    },
    {
      "version": 0,
      "type": 4,
      "position": 2,
      "permission_overwrites": [],
      "name": "Voice Channels",
      "id": "492122907300855818",
      "flags": 0
    },
    {
      "version": 0,
      "user_limit": 0,
      "type": 2,
      "rtc_region": null,
      "rate_limit_per_user": 0,
      "position": 0,
      "permission_overwrites": [],
      "parent_id": "492122907300855818",
      "name": "General",
      "last_message_id": null,
      "id": "492122907300855819",
      "flags": 0,
      "bitrate": 64000
    },
    {
      "version": 0,
      "type": 0,
      "topic": null,
      "rate_limit_per_user": 0,
      "position": 1,
      "permission_overwrites": [
        {
          "type": 0,
          "id": "780239802196426773",
          "deny": "1024",
          "allow": "0"
        }
      ],
      "parent_id": "492122906864779275",
      "name": "avalon_bot",
      "last_message_id": "849017251914842192",
      "id": "492122962451759124",
      "flags": 0
    },
    {
      "version": 1667238419348,
      "type": 0,
      "topic": null,
      "rate_limit_per_user": 0,
      "position": 2,
      "permission_overwrites": [
        {
          "type": 0,
          "id": "492123963728855044",
          "deny": "1024",
          "allow": "0"
        }
      ],
      "parent_id": "492122906864779275",
      "name": "dev_bot",
      "last_pin_timestamp": "2023-07-06T20:05:55+00:00",
      "last_message_id": "1131797440643735683",
      "id": "780240796690808912",
      "flags": 0
    },
    {
      "version": 0,
      "type": 0,
      "topic": "topic lol",
      "rate_limit_per_user": 0,
      "position": 3,
      "permission_overwrites": [
        {
          "type": 1,
          "id": "592500196303437826",
          "deny": "262144",
          "allow": "16"
        }
      ],
      "parent_id": "492122906864779275",
      "name": "test-member-overwrite",
      "last_message_id": "793351162396278814",
      "id": "781369592982929408",
      "flags": 0
    }
  ],
  "name": "Bots Bots Bots",
  "max_stage_video_channel_users": 50,
  "premium_tier": 0
}"#;
    let guild: Guild = serde_json::from_str(JSON).unwrap();
    println!("guild = {:#?}", guild);
}

#[async_trait]
impl Update for GuildCreate {
    async fn update(&self, cache: &Cache) {
        let (mut t, mut c, mut n) = (Vec::new(), Vec::new(), Vec::new());
        {
            let mut guard = cache.channel_types.write().await;
            self.guild.channels.iter()
                .for_each(|channel| {
                    guard.insert(channel.id(), channel.variant_type());
                    match channel {
                        Channel::Text(text) => {
                            let mut text = text.clone();
                            text.guild_id = Some(self.guild.id);
                            t.push(text);
                        }
                        Channel::Category(category) => {
                            let mut category = category.clone();
                            category.guild_id = Some(self.guild.id);
                            c.push(category);
                        }
                        Channel::Announcement(news) => {
                            let mut news = news.clone();
                            news.guild_id = Some(self.guild.id);
                            n.push(news);
                        }
                        // Channel::Store(store) => s.push(store.clone()),
                        Channel::Voice(_) => {
                            // not (yet/ever) implemented
                        }
                        Channel::Dm(_) | Channel::GroupDm(_) => {
                            // not part of a guild
                        }
                        // todo
                        Channel::AnnouncementThread(_) => {}
                        Channel::PublicThread(_) => {}
                        Channel::PrivateThread(_) => {}
                        Channel::GuildStageVoice(_) => {}
                        Channel::GuildDirectory(_) => {}
                        Channel::GuildForum(_) => {}
                    }
                });
        }
        cache.channels.write().await.extend(t);
        cache.categories.write().await.extend(c);
        cache.news.write().await.extend(n);
        // cache.stores.write().await.extend(s);

        let mut members = cache.members.write().await;
        for member in self.guild.members.iter().cloned() {
            let user_id = member.user.id;
            members.entry(user_id)
                .or_default()
                .insert(self.guild.id, member.clone());
            // only an `or_insert` because this is only a partial user
            cache.users.write().await.entry(user_id).or_insert(member.user);
        }
        cache.unavailable_guilds.write().await.remove(&self.guild);

        cache.guilds.write().await.insert(self.guild.clone());
    }
}

/// Sent when a [Guild](Guild) is updated.
#[derive(Deserialize, Debug, Clone)]
pub struct GuildUpdate {
    id: GuildId,
    name: Option<String>,
    icon: Option<String>,
    splash: Option<String>,
    discovery_splash: Option<String>,
    owner: Option<bool>,
    owner_id: UserId,
    permissions: Option<Permissions>,
    region: String,
    afk_channel_id: Option<ChannelId>,
    afk_timeout: u32,
    widget_enabled: Option<bool>,
    widget_channel_id: Option<ChannelId>,
    verification_level: VerificationLevel,
    default_message_notifications: NotificationLevel,
    explicit_content_filter: ExplicitFilterLevel,
    roles: IdMap<Role>,
    emojis: IdMap<CustomEmoji>,
    features: HashSet<GuildFeature>,
    mfa_level: MfaLevel,
    application_id: Option<ApplicationId>,
    system_channel_id: Option<ChannelId>,
    system_channel_flags: SystemChannelFlags,
    rules_channel_id: Option<ChannelId>,
    max_presences: Option<u32>,
    max_members: Option<u32>,
    vanity_url_code: Option<String>,
    description: Option<String>,
    banner: Option<String>,
    premium_tier: PremiumTier,
    premium_subscription_count: Option<u32>,
    preferred_locale: Option<String>,
    public_updates_id_channel: Option<ChannelId>,
    max_video_channel_users: Option<u32>,
    approximate_member_count: Option<u32>,
    approximate_presence_count: Option<u32>,
}

#[async_trait]
impl Update for GuildUpdate {
    async fn update(&self, cache: &Cache) {
        cache.guilds.write().await.entry(self.id).and_modify(|guild| {
            let s = self.clone();
            guild.id = s.id;
            guild.name = s.name;
            guild.icon = s.icon;
            guild.splash = s.splash;
            guild.discovery_splash = s.discovery_splash;
            guild.owner = s.owner.unwrap_or(guild.owner);
            guild.owner_id = s.owner_id;
            guild.permissions = s.permissions;
            guild.region = s.region;
            guild.afk_channel_id = s.afk_channel_id;
            guild.afk_timeout = s.afk_timeout;
            guild.widget_enabled = s.widget_enabled;
            guild.widget_channel_id = s.widget_channel_id;
            guild.verification_level = s.verification_level;
            guild.default_message_notifications = s.default_message_notifications;
            guild.explicit_content_filter = s.explicit_content_filter;
            guild.roles = s.roles;
            guild.emojis = s.emojis;
            guild.features = s.features;
            guild.mfa_level = s.mfa_level;
            guild.application_id = s.application_id;
            guild.system_channel_id = s.system_channel_id;
            guild.system_channel_flags = s.system_channel_flags;
            guild.rules_channel_id = s.rules_channel_id;
            guild.max_presences = s.max_presences;
            guild.max_members = s.max_members;
            guild.vanity_url_code = s.vanity_url_code;
            guild.description = s.description;
            guild.banner = s.banner;
            guild.premium_tier = s.premium_tier;
            guild.premium_subscription_count = s.premium_subscription_count;
            guild.preferred_locale = s.preferred_locale;
            guild.public_updates_id_channel = s.public_updates_id_channel;
            guild.max_video_channel_users = s.max_video_channel_users;
            guild.approximate_member_count = s.approximate_member_count;
            guild.approximate_presence_count = s.approximate_presence_count;
        });
    }
}

/// Sent when a guild becomes or was already unavailable due to an outage, or when the user leaves
/// or is removed from a guild. If the `unavailable` field is not set, the user was removed from the
/// guild.
#[derive(Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct GuildDelete {
    guild: UnavailableGuild,
}

#[async_trait]
impl Update for GuildDelete {
    async fn update(&self, cache: &Cache) {
        if let Some(guild) = cache.guilds.read().await.get(&self.guild) {
            {
                let mut guard = cache.channel_types.write().await;
                for channel in &guild.channels {
                    guard.remove(&channel.id());
                }
            }
            let mut guard = cache.members.write().await;
            guild.members.iter()
                .for_each(|m| {
                    guard.entry(m.user.id).and_modify(|map| {
                        map.remove(&guild.id);
                    });
                });
        }
        cache.guilds.write().await.remove(&self.guild);
        if self.guild.unavailable {
            cache.unavailable_guilds.write().await.insert(self.guild);
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct BanAdd {
    /// id of the guild
    pub guild_id: GuildId,
    /// the banned user
    pub user: User,
}

#[async_trait]
impl Update for BanAdd {
    async fn update(&self, _cache: &Cache) {
        // todo: cache bans?
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct BanRemove {
    /// id of the guild
    pub guild_id: GuildId,
    /// the unbanned user
    pub user: User,
}

#[async_trait]
impl Update for BanRemove {
    async fn update(&self, _cache: &Cache) {
        // todo: cache bans?
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct EmojiUpdate {
    /// id of the guild
    pub guild_id: GuildId,
    /// array of emojis
    pub emojis: IdMap<CustomEmoji>,
}

#[async_trait]
impl Update for EmojiUpdate {
    async fn update(&self, cache: &Cache) {
        println!("UPDATE emoji = {self:#?}");
        cache.guilds.write().await.entry(self.guild_id)
            .and_modify(|guild| self.emojis.iter()
                .cloned()
                .for_each(|emoji| guild.emojis.insert(emoji))
            );
    }
}

// why does this and GUILD_INTEGRATIONS_UPDATE exist? who knows
#[derive(Deserialize, Debug, Clone)]
pub struct IntegrationUpdate {
    pub guild_id: GuildId,
    #[serde(flatten)]
    pub integration: Integration,
}

#[async_trait]
impl Update for IntegrationUpdate {
    async fn update(&self, _cache: &Cache) {}
}

/// Sent when a guild integration is updated.
#[derive(Deserialize, Debug, Clone)]
pub struct GuildIntegrationsUpdate {
    /// id of the guild whose integrations were updated
    pub guild_id: GuildId,
}

#[async_trait]
impl Update for GuildIntegrationsUpdate {
    async fn update(&self, _cache: &Cache) {
        // nothing has to happen here
    }
}

/// Sent when a new user joins a guild.
///
/// [`GUILD_MEMBERS`](crate::shard::intents::Intents::GUILD_MEMBERS) is required to receive this.
#[derive(Deserialize, Debug, Clone)]
pub struct GuildMemberAdd {
    /// id of the guild
    pub guild_id: GuildId,
    #[serde(flatten)]
    pub member: GuildMember,
}

#[async_trait]
impl Update for GuildMemberAdd {
    async fn update(&self, cache: &Cache) {
        cache.members.write().await.entry(self.member.user.id)
            .and_modify(|map| {
                map.insert(self.guild_id, self.member.clone());
            });
        cache.guilds.write().await.entry(self.guild_id)
            .and_modify(|guild| guild.members.insert(self.member.clone()));
        cache.users.write().await.entry(&self.member).or_insert_with(|| self.member.user.clone());
    }
}

/// Sent when a user is removed from a guild (leave/kick/ban).
///
/// [`GUILD_MEMBERS`](crate::shard::intents::Intents::GUILD_MEMBERS) is required to receive this.
#[derive(Deserialize, Debug, Clone)]
pub struct GuildMemberRemove {
    /// the id of the guild
    pub guild_id: GuildId,
    /// the user who was removed
    pub user: User,
}

#[async_trait]
impl Update for GuildMemberRemove {
    async fn update(&self, cache: &Cache) {
        cache.members.write().await.entry(self.user.id)
            .and_modify(|map| {
                map.remove(&self.guild_id);
            });
        cache.guilds.write().await.entry(self.guild_id)
            .and_modify(|guild| { guild.members.remove(self.user.clone()); });
        // don't remove from `cache.users` because they could be in other guilds too or have a dm or w/e
    }
}

/// Sent when a guild member is updated. This will also fire when the user of a guild member changes.
///
/// The [`GUILD_MEMBERS`](crate::shard::intents::Intents::GUILD_MEMBERS) intent is required to
/// receive this.
#[derive(Deserialize, Debug, Clone)]
pub struct GuildMemberUpdate {
    /// the id of the guild
    pub guild_id: GuildId,
    /// user role ids
    pub roles: HashSet<RoleId>,
    /// the user
    pub user: User,
    /// nickname of the user in the guild
    pub nick: Option<String>,
    /// when the user joined the guild
    pub joined_at: DateTime<Utc>,
    /// when the user starting boosting the guild
    pub premium_since: Option<DateTime<Utc>>,
}

#[async_trait]
impl Update for GuildMemberUpdate {
    async fn update(&self, cache: &Cache) {
        println!("UPDATE guild_member = {self:?}");
        let mut guard = cache.members.write().await;
        let option = guard.get_mut(&self.user.id)
            .and_then(|map| map.get_mut(&self.guild_id));
        if let Some(member) = option {
            let new = self.clone();
            member.user = new.user;
            member.nick = new.nick;
            member.roles = new.roles;
            member.joined_at = new.joined_at;
            member.premium_since = new.premium_since;
        }

        if let Some(guild) = cache.guilds.write().await.get_mut(self.guild_id) {
            if let Some(member) = guild.members.get_mut(&self.user) {
                let s = self.clone();
                member.user = s.user;
                member.nick = s.nick;
                member.roles = s.roles;
                member.joined_at = s.joined_at;
                member.premium_since = s.premium_since;
            }
        }
    }
}

/// Sent in response to Guild Request Members. You can use `chunk_index` and `chunk_count` to
/// calculate how many chunks are left for your request.
#[derive(Deserialize, Debug, Clone)]
pub struct GuildMembersChunk {
    /// the id of the guild
    pub guild_id: GuildId,
    /// set of guild members
    pub members: IdMap<GuildMember>,
    /// the chunk index in the expected chunks for this response (0 <= chunk_index < chunk_count)
    pub chunk_index: u32,
    /// the total number of expected chunks for this response
    pub chunk_count: u32,
    // todo I think this is user id? could also be the guild id (or both ig)
    /// if passing an invalid id to REQUEST_GUILD_MEMBERS, it will be returned here
    #[serde(default)]
    pub not_found: Vec<UserId>,
    /// if passing true to REQUEST_GUILD_MEMBERS, presences of the returned members will be here
    #[serde(default)]
    pub presences: IdMap<PresenceUpdate>,
    /// the nonce used in the Guild Members Request
    pub nonce: Option<String>,
}

#[async_trait]
impl Update for GuildMembersChunk {
    async fn update(&self, cache: &Cache) {
        let mut guard = cache.members.write().await;
        for member in &self.members {
            let cached = guard.get_mut(&member.user.id)
                .and_then(|map| map.get_mut(&self.guild_id));
            if let Some(cached) = cached {
                *cached = member.clone();
            }
            cache.users.write().await.entry(member).or_insert_with(|| member.user.clone());
        }
        if let Some(guild) = cache.guilds.write().await.get_mut(self.guild_id) {
            guild.members.extend(self.members.clone());
            guild.presences.extend(self.presences.clone());
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct GuildRoleCreate {
    /// the id of the guild
    pub(crate) guild_id: GuildId,
    /// the role created
    pub(crate) role: Role,
}

#[async_trait]
impl Update for GuildRoleCreate {
    async fn update(&self, cache: &Cache) {
        if let Some(guild) = cache.guilds.write().await.get_mut(self.guild_id) {
            guild.roles.insert(self.role.clone());
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct GuildRoleUpdate {
    /// the id of the guild
    pub(crate) guild_id: GuildId,
    /// the role created
    pub(crate) role: Role,
}

#[async_trait]
impl Update for GuildRoleUpdate {
    async fn update(&self, cache: &Cache) {
        if let Some(guild) = cache.guilds.write().await.get_mut(self.guild_id) {
            guild.roles.insert(self.role.clone());
        }
    }
}

#[derive(Deserialize, Debug, Copy, Clone)]
pub struct GuildRoleDelete {
    /// the id of the guild
    pub(crate) guild_id: GuildId,
    /// the role created
    pub(crate) role_id: RoleId,
}

#[async_trait]
impl Update for GuildRoleDelete {
    async fn update(&self, cache: &Cache) {
        if let Some(guild) = cache.guilds.write().await.get_mut(self.guild_id) {
            guild.roles.remove(self.role_id);
        }
    }
}

// Invite Events

#[derive(Deserialize, Debug, Clone)]
pub struct InviteCreate {
    /// the channel the invite is for
    pub channel_id: ChannelId,
    /// the unique invite code
    pub code: String,
    /// the time at which the invite was created
    pub created_at: DateTime<Utc>,
    /// the guild of the invite
    pub guild_id: Option<GuildId>,
    /// the user that created the invite
    pub inviter: Option<User>,
    /// how long the invite is valid for (in seconds)
    // todo deserialize as Duration
    pub max_age: i32,
    /// the maximum number of times the invite can be used
    pub max_uses: u32,
    /// the target user for this invite
    pub target_user: Option<User>,
    /// the type of user target for this invite
    // todo model Invite: https://discord.com/developers/docs/resources/invite#invite-object-target-user-types
    pub target_user_type: Option<u8>,
    /// whether or not the invite is temporary (invited users will be kicked on disconnect unless they're assigned a role)
    pub temporary: bool,
    /// how many times the invite has been used (always will be 0)
    pub uses: u8,
}

#[async_trait]
impl Update for InviteCreate {
    async fn update(&self, _cache: &Cache) {}
}

#[derive(Deserialize, Debug, Clone)]
pub struct InviteDelete {
    /// the channel of the invite
    pub channel_id: ChannelId,
    /// the guild of the invite
    pub guild_id: Option<GuildId>,
    /// the unique invite [code](https://discord.com/developers/docs/resources/invite#invite-object)
    pub code: String,
}

#[async_trait]
impl Update for InviteDelete {
    async fn update(&self, _cache: &Cache) {}
}

// Message Events

#[derive(Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct MessageCreate {
    pub(crate) message: Message,
}

#[async_trait]
impl Update for MessageCreate {
    async fn update(&self, cache: &Cache) {
        let channel_type = cache.channel_types.read().await
            .get(&self.message.channel)
            .copied();
        match channel_type {
            Some(ChannelType::Text) => {
                // todo should this just unwrap?
                if let Some(channel) = cache.channels.write().await.get_mut(&self.message.channel) {
                    channel.last_message_id = Some(self.message.id);
                }
            }
            Some(ChannelType::Dm) => {
                if let Some(dm) = cache.dms.write().await.1.get_mut(&self.message.channel) {
                    dm.last_message_id = Some(self.message.id);
                }
            }
            Some(ChannelType::Announcement) => {
                if let Some(news) = cache.news.write().await.get_mut(&self.message.channel) {
                    news.last_message_id = Some(self.message.id);
                }
            }
            Some(ChannelType::Voice
                 | ChannelType::GroupDm
                 | ChannelType::Category)
            | None => {}
            // todo
            Some(ChannelType::AnnouncementThread) => {}
            Some(ChannelType::PublicThread) => {}
            Some(ChannelType::PrivateThread) => {}
            Some(ChannelType::GuildStageVoice) => {}
            Some(ChannelType::GuildDirectory) => {}
            Some(ChannelType::GuildForum) => {}
        }
        cache.users.write().await.insert(self.message.author.clone());
        cache.messages.write().await.insert(self.message.clone());
        if let Some(interaction) = self.message.interaction.clone() {
            cache.interaction_responses.write().await.insert(interaction.id, self.message.clone());
        }
    }
}

/// like `Message` but everything (except for `id`, `channel_id`) is optional
#[derive(Deserialize, Debug, Clone)]
pub struct MessageUpdate {
    pub(crate) id: MessageId,
    pub(crate) channel_id: ChannelId,
    // pub(crate) guild_id: Option<Option<GuildId>>,
    pub(crate) author: Option<User>,
    // pub(crate) member: Option<Option<GuildMemberUserless>>,
    pub(crate) content: Option<String>,
    pub(crate) timestamp: Option<DateTime<Utc>>,
    pub(crate) edited_timestamp: Option<Option<DateTime<Utc>>>,
    pub(crate) tts: Option<bool>,
    pub(crate) mention_everyone: Option<bool>,
    pub(crate) mentions: Option<Vec<User>>,
    pub(crate) mention_roles: Option<Vec<RoleId>>,
    pub(crate) mention_channels: Option<Vec<ChannelMention>>,
    pub(crate) attachments: Option<Vec<Attachment>>,
    pub(crate) embeds: Option<Vec<Embed>>,
    pub(crate) reactions: Option<Vec<Reaction>>,
    pub(crate) nonce: Option<Option<String>>,
    pub(crate) pinned: Option<bool>,
    pub(crate) webhook_id: Option<Option<WebhookId>>,
    #[serde(rename = "type")]
    pub(crate) message_type: Option<MessageType>,
    pub(crate) activity: Option<Option<MessageActivity>>,
    pub(crate) application: Option<Option<MessageApplication>>,
    pub(crate) application_id: Option<Option<ApplicationId>>,
    pub(crate) message_reference: Option<Option<MessageReference>>,
    pub(crate) flags: Option<MessageFlags>,
    pub(crate) referenced_message: Option<Option<Message>>,
    pub(crate) interaction: Option<Option<MessageInteraction>>,
    pub(crate) thread: Option<Option<Channel>>,
    pub(crate) components: Option<Vec<ActionRow>>,
    pub(crate) sticker_items: Option<Vec<StickerItem>>,
    pub(crate) position: Option<Option<usize>>,
}

impl TryFrom<MessageUpdate> for Message {
    type Error = ();

    fn try_from(update: MessageUpdate) -> Result<Self, Self::Error> {
        // todo look at this again, looks a bit wrong
        fn option(update: MessageUpdate) -> Option<Message> {
            Some(Message {
                id: update.id,
                channel: update.channel_id,
                // guild_id: update.guild_id.unwrap_or_default(),
                author: update.author?,
                // member: update.member.unwrap_or_default(),
                content: update.content?,
                timestamp: update.timestamp?,
                edited_timestamp: update.edited_timestamp.unwrap_or_default(),
                tts: update.tts.unwrap_or_default(),
                mention_everyone: update.mention_everyone.unwrap_or_default(),
                mentions: update.mentions.unwrap_or_default(),
                mention_roles: update.mention_roles.unwrap_or_default(),
                mention_channels: update.mention_channels.unwrap_or_default(),
                attachments: update.attachments.unwrap_or_default(),
                embeds: update.embeds.unwrap_or_default(),
                reactions: update.reactions.unwrap_or_default(),
                nonce: update.nonce.unwrap_or_default(),
                pinned: update.pinned.unwrap_or_default(),
                webhook_id: update.webhook_id.unwrap_or_default(),
                message_type: update.message_type?,
                activity: update.activity.unwrap_or_default(),
                application: update.application.unwrap_or_default(),
                application_id: update.application_id.unwrap_or_default(),
                message_reference: update.message_reference.unwrap_or_default(),
                flags: update.flags.unwrap_or_default(),
                referenced_message: update.referenced_message.unwrap_or_default().map(Box::new),
                interaction: update.interaction.unwrap_or_default(),
                thread: update.thread.unwrap_or_default(),
                components: update.components.unwrap_or_default(),
                sticker_items: update.sticker_items.unwrap_or_default(),
                position: update.position.unwrap_or_default(),
            })
        }
        option(update).ok_or(())
    }
}

#[async_trait]
impl Update for MessageUpdate {
    async fn update(&self, cache: &Cache) {
        if let Some(author) = self.author.clone() {
            cache.users.write().await.insert(author);
        }
        let mut guard = cache.messages.write().await;
        // don't forget that the lock on `channel_types` is held throughout all branches
        #[allow(clippy::significant_drop_in_scrutinee)]
        match guard.entry(self.id) {
            Entry::Occupied(mut e) => {
                fn update<T>(original: &mut T, new: Option<T>) {
                    if let Some(new) = new { *original = new; }
                }
                let message = e.get_mut();
                let s = self.clone();
                // update(&mut message.guild_id, s.guild_id);
                update(&mut message.author, s.author);
                // update(&mut message.member, s.member);
                update(&mut message.content, s.content);
                update(&mut message.edited_timestamp, s.edited_timestamp);
                update(&mut message.tts, s.tts);
                update(&mut message.mention_everyone, s.mention_everyone);
                update(&mut message.mentions, s.mentions);
                update(&mut message.mention_roles, s.mention_roles);
                update(&mut message.mention_channels, s.mention_channels);
                update(&mut message.attachments, s.attachments);
                update(&mut message.embeds, s.embeds);
                update(&mut message.reactions, s.reactions);
                update(&mut message.nonce, s.nonce);
                update(&mut message.pinned, s.pinned);
                update(&mut message.webhook_id, s.webhook_id);
                update(&mut message.message_type, s.message_type);
                update(&mut message.activity, s.activity);
                update(&mut message.application, s.application);
                update(&mut message.message_reference, s.message_reference);
                update(&mut message.flags, s.flags);
                if let Some(referenced) = s.referenced_message {
                    message.referenced_message = referenced.map(Box::new);
                }
            }
            Entry::Vacant(e) => {
                if let Ok(message) = self.clone().try_into() {
                    e.insert(message);
                }
            }
        };
    }
}

#[derive(Deserialize, Debug, Copy, Clone)]
pub struct MessageDelete {
    /// the id of the message
    pub id: MessageId,
    /// the id of the channel
    pub channel_id: ChannelId,
    /// the id of the guild
    pub guild_id: Option<GuildId>,
}

#[async_trait]
impl Update for MessageDelete {
    async fn update(&self, cache: &Cache) {
        cache.messages.write().await.remove(self.id);
        // don't forget that the lock on `channel_types` is held throughout all branches
        #[allow(clippy::significant_drop_in_scrutinee)]
        match cache.channel_types.read().await.get(&self.channel_id) {
            Some(ChannelType::Text) => {
                if let Some(channel) = cache.channels.write().await.get_mut(self.channel_id) {
                    channel.last_message_id = channel.last_message_id.filter(|&id| id != self.id);
                }
            }
            Some(ChannelType::Dm) => {
                if let Some(channel) = cache.dms.write().await.1.get_mut(self.channel_id) {
                    channel.last_message_id = channel.last_message_id.filter(|&id| id != self.id);
                }
            }
            Some(ChannelType::Announcement) => {
                if let Some(channel) = cache.news.write().await.get_mut(self.channel_id) {
                    channel.last_message_id = channel.last_message_id.filter(|&id| id != self.id);
                }
            }
            Some(_) | None => {}
        }
        if let Some(id) = self.guild_id {
            if let Some(guild) = cache.guilds.write().await.get_mut(id) {
                match guild.channels.get_mut(self.channel_id) {
                    Some(Channel::Text(text)) => {
                        text.last_message_id = text.last_message_id.filter(|&id| id != self.id);
                    }
                    Some(Channel::Dm(dm)) => {
                        dm.last_message_id = dm.last_message_id.filter(|&id| id != self.id);
                    }
                    Some(Channel::Announcement(news)) => {
                        news.last_message_id = news.last_message_id.filter(|&id| id != self.id);
                    }
                    Some(_) | None => {}
                }
            }
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct MessageDeleteBulk {
    /// the id of the message
    pub ids: Vec<MessageId>,
    /// the id of the channel
    pub channel_id: ChannelId,
    /// the id of the guild
    pub guild_id: Option<GuildId>,
}

#[async_trait]
impl Update for MessageDeleteBulk {
    async fn update(&self, cache: &Cache) {
        let Self { ids, channel_id, guild_id } = self;
        let (channel_id, guild_id) = (*channel_id, *guild_id);
        futures::stream::iter(ids.iter().copied())
            .map(|id| MessageDelete { id, channel_id, guild_id })
            .for_each(|delete| async move { delete.update(cache).await })
            .await;
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ReactionAdd {
    /// the id of the user
    pub user_id: UserId,
    /// the id of the channel
    pub channel_id: ChannelId,
    /// the id of the message
    pub message_id: MessageId,
    /// the id of the guild
    pub guild_id: Option<GuildId>,
    /// the member who reacted if this happened in a guild
    pub member: Option<GuildMember>,
    /// the emoji used to react
    pub emoji: Emoji,
}

#[async_trait]
impl Update for ReactionAdd {
    async fn update(&self, cache: &Cache) {
        if let Some(message) = cache.messages.write().await.get_mut(self.message_id) {
            let idx = message.reactions.iter()
                .position(|reaction| reaction.emoji == self.emoji);
            let me = cache.user.read().await
                .as_ref()
                .map_or(false, |me| me.id == self.user_id);
            if let Some(idx) = idx {
                let reaction = &mut message.reactions[idx];
                reaction.count += 1;
                reaction.me |= me;
            } else {
                message.reactions.push(Reaction {
                    count: 1,
                    me,
                    emoji: self.emoji.clone(),
                });
            }
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ReactionRemove {
    /// the id of the user
    pub user_id: UserId,
    /// the id of the channel
    pub channel_id: ChannelId,
    /// the id of the message
    pub message_id: MessageId,
    /// the id of the guild
    pub guild_id: Option<GuildId>,
    /// the emoji used to react
    pub emoji: Emoji,
}

#[async_trait]
impl Update for ReactionRemove {
    async fn update(&self, cache: &Cache) {
        if let Some(message) = cache.messages.write().await.get_mut(self.message_id) {
            let idx = message.reactions.iter()
                .position(|reaction| reaction.emoji == self.emoji);
            let me = cache.user.read().await
                .as_ref()
                .map_or(false, |me| me.id == self.user_id);
            if let Some(idx) = idx {
                let reaction = &mut message.reactions[idx];
                reaction.count -= 1;
                reaction.me &= !me;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ReactionType { Add, Remove }

#[derive(Debug, Clone)]
pub struct ReactionUpdate {
    /// whether this reaction was added or removed
    pub kind: ReactionType,
    /// the id of the user
    pub user_id: UserId,
    /// the id of the channel
    pub channel_id: ChannelId,
    /// the id of the message
    pub message_id: MessageId,
    /// the id of the guild
    pub guild_id: Option<GuildId>,
    /// the emoji used to react
    pub emoji: Emoji,
}

impl ReactionUpdate {
    pub fn channel_message(&self) -> ChannelMessageId {
        ChannelMessageId {
            channel: self.channel_id,
            message: self.message_id,
        }
    }
}

impl From<ReactionAdd> for ReactionUpdate {
    fn from(add: ReactionAdd) -> Self {
        Self {
            kind: ReactionType::Add,
            user_id: add.user_id,
            channel_id: add.channel_id,
            message_id: add.message_id,
            guild_id: add.guild_id,
            emoji: add.emoji,
        }
    }
}

impl From<ReactionRemove> for ReactionUpdate {
    fn from(remove: ReactionRemove) -> Self {
        Self {
            kind: ReactionType::Remove,
            user_id: remove.user_id,
            channel_id: remove.channel_id,
            message_id: remove.message_id,
            guild_id: remove.guild_id,
            emoji: remove.emoji,
        }
    }
}

/// Sent when a user explicitly removes all reactions from a message.
#[derive(Deserialize, Debug, Copy, Clone)]
pub struct ReactionRemoveAll {
    /// the id of the channel
    pub channel_id: ChannelId,
    /// the id of the message
    pub message_id: MessageId,
    /// the id of the guild
    pub guild_id: Option<GuildId>,
}

#[async_trait]
impl Update for ReactionRemoveAll {
    async fn update(&self, cache: &Cache) {
        if let Some(message) = cache.messages.write().await.get_mut(self.message_id) {
            message.reactions.clear();
        }
    }
}

/// Sent when a bot removes all instances of a given emoji from the reactions of a message.
#[derive(Deserialize, Debug, Clone)]
pub struct ReactionRemoveEmoji {
    /// the id of the channel
    pub channel_id: ChannelId,
    /// the id of the message
    pub message_id: MessageId,
    /// the id of the guild
    pub guild_id: Option<GuildId>,
    /// the emoji that was removed
    pub emoji: Emoji,
}

#[async_trait]
impl Update for ReactionRemoveEmoji {
    async fn update(&self, cache: &Cache) {
        if let Some(message) = cache.messages.write().await.get_mut(self.message_id) {
            message.reactions.retain(|f| f.emoji != self.emoji);
        }
    }
}

// Presence Updates

/// A user's presence is their current state on a guild. This event is sent when a user's presence
/// or info, such as name or avatar, is updated.
///
/// [`GUILD_PRESENCES`](crate::shard::intents::Intents::GUILD_PRESENCES) is required to receive this.
///
/// The user object within this event can be partial, the only field which must be sent is the `id`
/// field, everything else is optional. Along with this limitation, no fields are required, and the
/// types of the fields are **not** validated. Your client should expect any combination of fields
/// and types within this event.
// todo ^ that's a bit scary
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PresenceUpdate {
    /// the user presence is being updated for
    pub user: User,
    /// id of the guild
    pub guild_id: GuildId,
    /// either "idle", "dnd", "online", or "offline"
    pub status: StatusType,
    /// user's current activities
    pub activities: Vec<Activity>,
    /// user's platform-dependent status
    pub client_status: ClientStatus,
}

impl PartialEq for PresenceUpdate {
    fn eq(&self, other: &Self) -> bool {
        self.user.id == other.user.id &&
            self.guild_id == other.guild_id
    }
}

impl Id for PresenceUpdate {
    type Id = UserId;

    fn id(&self) -> Self::Id {
        self.user.id
    }
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone)]
pub struct ClientStatus {
    /// the user's status set for an active desktop (Windows, Linux, Mac) application session
    pub desktop: Option<StatusType>,
    /// the user's status set for an active mobile (iOS, Android) application session
    pub mobile: Option<StatusType>,
    /// the user's status set for an active web (browser, bot account) application session
    pub web: Option<StatusType>,
}

#[async_trait]
impl Update for PresenceUpdate {
    async fn update(&self, cache: &Cache) {
        cache.users.write().await.insert(self.user.clone());
        if let Some(guild) = cache.guilds.write().await.get_mut(self.guild_id) {
            guild.presences.insert(self.clone());
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct TypingStart {
    /// id of the channel
    pub channel_id: ChannelId,
    /// id of the guild
    pub guild_id: Option<GuildId>,
    /// id of the user
    pub user_id: UserId,
    // todo Deserialize as DateTime<Utc>
    /// unix time (in seconds) of when the user started typing
    pub timestamp: u64,
    /// the member who started typing if this happened in a guild
    pub member: Option<GuildMember>,
}

#[async_trait]
impl Update for TypingStart {
    async fn update(&self, _cache: &Cache) {
        // don't think we need to update anything here?
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct UserUpdate {
    user: User,
}

#[async_trait]
impl Update for UserUpdate {
    async fn update(&self, cache: &Cache) {
        // todo make sure this does mean current user
        log::warn!("{:?}", &self);
        *cache.user.write().await = Some(self.user.clone());
        cache.users.write().await.insert(self.user.clone());
    }
}

// Voice Updates

#[derive(Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct VoiceStateUpdate {
    state: VoiceState,
}

#[async_trait]
impl Update for VoiceStateUpdate {
    async fn update(&self, cache: &Cache) {
        if let Some(guild_id) = self.state.guild_id {
            if let Some(map) = cache.members.write().await.get_mut(&self.state.user_id) {
                if let Some(member) = map.get_mut(&guild_id) {
                    member.deaf = self.state.self_deaf;
                    member.mute = self.state.self_mute;
                }
            }
            if let Some(guild) = cache.guilds.write().await.get_mut(guild_id) {
                guild.voice_states.insert(self.state.clone());
            }
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct VoiceServerUpdate {
    /// voice connection token
    pub token: String,
    /// the guild this voice server update is for
    pub guild_id: GuildId,
    /// the voice server host
    pub endpoint: String,
}

#[async_trait]
impl Update for VoiceServerUpdate {
    async fn update(&self, _cache: &Cache) {}
}

// Webhook Updates

#[derive(Deserialize, Debug, Clone)]
pub struct WebhookUpdate {
    /// id of the guild
    pub guild_id: GuildId,
    /// id of the channel
    pub channel_id: ChannelId,
}

#[async_trait]
impl Update for WebhookUpdate {
    async fn update(&self, _cache: &Cache) {}
}

// Slash Command Updates

#[derive(Deserialize, Debug, Clone)]
#[serde(transparent)]
pub struct InteractionCreate {
    pub(crate) interaction: Interaction,
}

#[async_trait]
impl Update for InteractionCreate {
    async fn update(&self, _cache: &Cache) {
        // println!("self = {:#?}", self);
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApplicationCommandCreate {
    pub guild_id: GuildId,
    #[serde(flatten)]
    pub command: InteractionData<ApplicationCommandData>,
}

#[async_trait]
impl Update for ApplicationCommandCreate {
    async fn update(&self, _cache: &Cache) {}
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApplicationCommandUpdate {
    pub guild_id: GuildId,
    #[serde(flatten)]
    pub command: InteractionData<ApplicationCommandData>,
}

#[async_trait]
impl Update for ApplicationCommandUpdate {
    async fn update(&self, _cache: &Cache) {}
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApplicationCommandDelete {
    pub guild_id: GuildId,
    #[serde(flatten)]
    pub command: InteractionData<ApplicationCommandData>,
}

#[async_trait]
impl Update for ApplicationCommandDelete {
    async fn update(&self, _cache: &Cache) {}
}

#[derive(Deserialize, Debug, Clone)]
pub struct ApplicationCommandPermissionsUpdate {
    pub application_id: ApplicationId,
    pub guild_id: GuildId,
    pub id: CommandId,
    // todo it could also be GuildPermissions :)
    // todo
    // pub permissions: Vec<CommandPermissions>,
}

#[async_trait]
impl Update for ApplicationCommandPermissionsUpdate {
    async fn update(&self, _cache: &Cache) {}
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct AutoModerationRuleCreate {
    #[serde(flatten)]
    rule: AutoModRule,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct AutoModerationRuleUpdate {
    #[serde(flatten)]
    rule: AutoModRule,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct AutoModerationRuleDelete {
    #[serde(flatten)]
    rule: AutoModRule,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct AutoModerationActionExecution {
    /// ID of the guild in which action was executed
    pub guild_id: GuildId,
    /// Action which was executed
    pub action: Action,
    /// ID of the rule which action belongs to
    pub rule_id: RuleId,
    /// Trigger type of rule which was triggered
    pub rule_trigger_type: TriggerType,
    /// ID of the user which generated the content which triggered the rule
    pub user_id: UserId,
    /// ID of the channel in which user content was posted
    pub channel_id: Option<ChannelId>,
    /// ID of any user message which content belongs to
    ///
    /// will not exist if message was blocked by automod or content was not part of any message
    pub message_id: Option<MessageId>,
    /// ID of any system auto moderation messages posted as a result of this action
    ///
    /// will not exist if this event does not correspond to an action with type SEND_ALERT_MESSAGE
    pub alert_system_message_id: Option<MessageId>,
    /// User-generated text content
    ///
    /// MESSAGE_CONTENT (1 << 15) gateway intent is required to receive the content and matched_content fields
    pub content: Option<String>,
    /// Word or phrase configured in the rule that triggered the rule
    pub matched_keyword: Option<String>,
    /// Substring in content that triggered the rule
    ///
    /// MESSAGE_CONTENT (1 << 15) gateway intent is required to receive the content and matched_content fields
    pub matched_content: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct ThreadCreate {
    // todo also has a `newly_created` boolean field
    #[serde(flatten)]
    thread: Thread,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct ThreadUpdate {
    #[serde(flatten)]
    thread: Thread,
}

#[derive(Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct ThreadDelete {
    id: ChannelId,
    guild_id: GuildId,
    parent_id: Option<ChannelId>,
    #[serde(rename = "type")]
    kind: ChannelType,
}

/// Sent when the current user gains access to a channel.
#[derive(Deserialize, Debug, Clone)]
pub struct ThreadListSync {
    /// ID of the guild
    pub guild_id: GuildId,
    /// Parent channel IDs whose threads are being synced. If omitted, then threads were synced for the entire guild. This array may contain channel_ids that have no active threads as well, so you know to clear that data.
    #[serde(default)]
    pub channel_ids: Vec<ChannelId>,
    /// All active threads in the given channels that the current user can access
    pub threads: Vec<Thread>,
    /// All thread member objects from the synced threads for the current user, indicating which threads the current user has been added to
    pub members: Vec<ThreadMember>,
}

/// Sent when the thread member object for the current user is updated.
#[derive(Deserialize, Debug, Clone)]
pub struct ThreadMemberUpdate {}

#[derive(Deserialize, Debug, Clone)]
pub struct ThreadMembersUpdate {}

#[derive(Deserialize, Debug, Clone)]
pub struct StickerUpdate {}

#[derive(Deserialize, Debug, Clone)]
pub struct GuildScheduledEventCreate {}

#[derive(Deserialize, Debug, Clone)]
pub struct GuildScheduledEventUpdate {}

#[derive(Deserialize, Debug, Clone)]
pub struct GuildScheduledEventDelete {}

#[derive(Deserialize, Debug, Clone)]
pub struct GuildScheduledEventUserAdd {}

#[derive(Deserialize, Debug, Clone)]
pub struct GuildScheduledEventUserRemove {}

#[derive(Deserialize, Debug, Clone)]
pub struct IntegrationCreate {}

#[derive(Deserialize, Debug, Clone)]
pub struct IntegrationDelete {}

#[derive(Deserialize, Debug, Clone)]
pub struct StageInstanceCreate {}

#[derive(Deserialize, Debug, Clone)]
pub struct StageInstanceUpdate {}

#[derive(Deserialize, Debug, Clone)]
pub struct StageInstanceDelete {}
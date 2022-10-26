bitflags! {
    /// Discord's Gateway Intents, which allow bots to opt in or out of receiving certain events.
    /// Using [Identify](crate::shard::model::Identify)'s [new](crate::shard::model::Identify::new)
    /// constructor, all non-privileged intents are sent. To specify different intents, use
    /// [Identify::intents](crate::shard::model::Identify::intents).
    ///
    /// See [Discord's documentation](https://discord.com/developers/docs/topics/gateway#gateway-intents) for more details.
    pub struct Intents: u32 {
        const GUILDS = 1 << 0;
        const GUILD_MEMBERS = 1 << 1;
        const GUILD_BANS = 1 << 2;
        const GUILD_EMOJIS = 1 << 3;
        const GUILD_INTEGRATIONS = 1 << 4;
        const GUILD_WEBHOOKS = 1 << 5;
        const GUILD_INVITES = 1 << 6;
        const GUILD_VOICE_STATES = 1 << 7;
        const GUILD_PRESENCES = 1 << 8;
        const GUILD_MESSAGES = 1 << 9;
        const GUILD_MESSAGE_REACTIONS = 1 << 10;
        const GUILD_MESSAGE_TYPING = 1 << 11;
        const DIRECT_MESSAGES = 1 << 12;
        const DIRECT_MESSAGE_REACTIONS = 1 << 13;
        const DIRECT_MESSAGE_TYPING = 1 << 14;
        const MESSAGE_CONTENT = 1 << 15;
        const GUILD_SCHEDULED_EVENTS = 1 << 16;
        const AUTO_MODERATION_CONFIGURATION = 1 << 20;
        const AUTO_MODERATION_ACTION_EXECUTION = 1 << 21;

        const PRIVELEGED = Self::GUILD_PRESENCES.bits | Self::GUILD_MEMBERS.bits | Self::MESSAGE_CONTENT.bits;
    }
}
serde_bitflag!(Intents: u32);
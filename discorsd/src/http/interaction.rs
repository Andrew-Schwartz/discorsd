//! A TON of stuff.

use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt::Debug;
use std::str::FromStr;

use serde::Serialize;

use crate::BotState;
use crate::commands::{ButtonCommand, MenuCommand, MenuData};
use crate::http::{ClientResult, DiscordClient};
use crate::http::channel::{embed, MessageAttachment, RichEmbed};
use crate::http::routes::Route::*;
use crate::model::components::{ActionRow, Button, Component, Menu};
use crate::model::ids::*;
use crate::model::interaction_response::{InteractionMessage, InteractionResponse};
use crate::model::message::{AllowedMentions, Message, MessageFlags};
use crate::model::new_command;
use crate::model::new_command::{ApplicationCommand, Command};
use crate::model::new_interaction::Token;

impl DiscordClient {
    /// Fetch all of the global commands for your application.
    ///
    /// # Errors
    ///
    /// If the http request fails, or fails to deserialize the response into a `Vec<ApplicationCommand>`
    pub async fn get_global_commands(&self, application: ApplicationId) -> ClientResult<Vec<ApplicationCommand>> {
        self.get(GetGlobalCommands(application)).await
    }

    /// Create a new global command. New global commands will be available in all guilds after 1 hour.
    ///
    /// Creating a command with the same name as an existing command for your application will
    /// overwrite the old command.
    ///
    /// # Errors
    ///
    /// If the http request fails, or fails to deserialize the response into a `ApplicationCommand`
    pub async fn create_global_command(
        &self,
        application: ApplicationId,
        command: Command,
    ) -> ClientResult<ApplicationCommand> {
        self.post(CreateGlobalCommand(application), command).await
    }

    /// Fetch a global command for your application.
    ///
    /// # Errors
    ///
    /// If the http request fails, or fails to deserialize the response into a `ApplicationCommand`
    pub async fn get_global_command(&self, application: ApplicationId, command: CommandId) -> ClientResult<ApplicationCommand> {
        self.get(GetGlobalCommand(application, command)).await
    }

    /// Edit a global command. Updates will be available in all guilds after 1 hour.
    ///
    /// # Errors
    ///
    /// If the http request fails, or fails to deserialize the response into a `ApplicationCommand`
    pub async fn edit_global_command<'a>(
        &self,
        application: ApplicationId,
        id: CommandId,
        // todo maybe don't support this it kinda breaks stuff
        new_name: Option<&'a str>,
        new_description: Option<&'a str>,
        new_options: Option<Vec<new_command::CommandOption>>,
        new_default_permission: Option<bool>,
    ) -> ClientResult<ApplicationCommand> {
        self.patch(
            EditGlobalCommand(application, id),
            Edit {
                name: new_name,
                description: new_description,
                options: new_options,
                default_permission: new_default_permission,
            },
        ).await
    }

    /// Deletes a global command.
    ///
    /// # Errors
    ///
    /// If the http request fails
    pub async fn delete_global_command(
        &self,
        application: ApplicationId,
        id: CommandId,
    ) -> ClientResult<()> {
        self.delete(DeleteGlobalCommand(application, id)).await
    }

    /// Takes a vec of application commands, overwriting ALL existing commands that are registered
    /// globally for this application. Updates will be available in all guilds after 1 hour.
    ///
    /// Commands that do not already exist will count toward daily application command create
    /// limits.
    ///
    /// Note: This will overwrite all types of application commands: slash commands, user commands,
    /// and message commands.
    ///
    /// # Errors
    ///
    /// If the http request fails, or fails to deserialize the response into a
    /// `Vec<ApplicationCommand>`
    pub async fn bulk_overwrite_global_commands(
        &self,
        application: ApplicationId,
        commands: Vec<Command>,
    ) -> ClientResult<Vec<ApplicationCommand>> {
        // ) -> ClientResult<Vec<InteractionData<ApplicationCommandData>>> {
        self.put(BulkOverwriteGlobalCommands(application), commands).await
    }

    /// Fetch all of the guild commands for your application for a specific guild.
    ///
    /// # Errors
    ///
    /// If the http request fails, or fails to deserialize the response into a `Vec<ApplicationCommand>`
    pub async fn get_guild_commands(&self, application: ApplicationId, guild: GuildId) -> ClientResult<Vec<ApplicationCommand>> {
        self.get(GetGuildCommands(application, guild)).await
    }

    /// Create a new guild command. New guild commands will be available in the guild immediately.
    ///
    /// Creating a command with the same name as an existing command for your application will
    /// overwrite the old command.
    ///
    /// # Errors
    ///
    /// If the http request fails, or fails to deserialize the response into a `ApplicationCommand`
    pub async fn create_guild_command(
        &self,
        application: ApplicationId,
        guild: GuildId,
        command: Command,
    ) -> ClientResult<ApplicationCommand> {
        self.post(CreateGuildCommand(application, guild), command).await
    }

    /// Fetch a guild command for your application.
    ///
    /// # Errors
    ///
    /// If the http request fails, or fails to deserialize the response into a `ApplicationCommand`
    pub async fn get_guild_command(
        &self,
        application: ApplicationId,
        guild: GuildId,
        command: CommandId,
    ) -> ClientResult<ApplicationCommand> {
        self.get(GetGuildCommand(application, guild, command)).await
    }

    /// Edit a guild command. Updates for guild commands will be available immediately.
    ///
    /// # Errors
    ///
    /// If the http request fails, or fails to deserialize the response into a `ApplicationCommand`
    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub async fn edit_guild_command<'a>(
        &self,
        application: ApplicationId,
        guild: GuildId,
        id: CommandId,
        new_name: Option<&'a str>,
        new_description: Option<&'a str>,
        new_options: Option<Vec<new_command::CommandOption>>,
        new_default_permission: Option<bool>,
    ) -> ClientResult<ApplicationCommand> {
        self.patch(
            EditGuildCommand(application, guild, id),
            Edit {
                name: new_name,
                description: new_description,
                options: new_options,
                default_permission: new_default_permission,
            },
        ).await
    }

    /// Delete a guild command.
    ///
    /// # Errors
    ///
    /// If the http request fails
    pub async fn delete_guild_command(
        &self,
        application: ApplicationId,
        guild: GuildId,
        id: CommandId,
    ) -> ClientResult<()> {
        self.delete(DeleteGuildCommand(application, guild, id)).await
    }

    /// Takes a vec of application commands, overwriting ALL existing commands for the guild.
    ///
    /// Note: This will overwrite all types of application commands: slash commands, user commands,
    /// and message commands.
    ///
    /// # Errors
    ///
    /// If the http request fails, or fails to deserialize the response into a `Vec<ApplicationCommand>`
    pub async fn bulk_overwrite_guild_commands(
        &self,
        application: ApplicationId,
        guild: GuildId,
        commands: Vec<Command>,
    ) -> ClientResult<Vec<ApplicationCommand>> {
        self.put(BulkOverwriteGuildCommands(application, guild), commands).await
    }

    // todo
    // /// Fetches command permissions for all commands for your application in a guild.
    // ///
    // /// # Errors
    // ///
    // /// If the http request fails, or fails to deserialize the response into a
    // /// `Vec<GuildApplicationCommandPermission>`
    // pub async fn get_guild_application_command_permissions(
    //     &self,
    //     application: ApplicationId,
    //     guild: GuildId,
    // ) -> ClientResult<Vec<GuildApplicationCommandPermission>> {
    //     self.get(GetGuildApplicationCommandPermissions(application, guild)).await
    // }

    // todo
    // /// Fetches command permissions for a specific command for your application in a guild.
    // ///
    // /// # Errors
    // ///
    // /// If the http request fails, or fails to deserialize the response into a
    // /// `GuildApplicationCommandPermission`
    // pub async fn get_application_command_permissions(
    //     &self,
    //     application: ApplicationId,
    //     guild: GuildId,
    //     command: CommandId,
    // ) -> ClientResult<GuildApplicationCommandPermission> {
    //     self.get(GetApplicationCommandPermissions(application, guild, command)).await
    // }

    // bots can't edit perms anymore :(
    // /// Edits command permissions for a specific command for your application in a guild.
    // ///
    // /// This endpoint will overwrite existing permissions for the command in that guild.
    // ///
    // /// # Errors
    // ///
    // /// If the http request fails
    // pub async fn edit_application_command_permissions(
    //     &self,
    //     application: ApplicationId,
    //     guild: GuildId,
    //     command: CommandId,
    //     permissions: Vec<CommandPermissions>,
    // ) -> ClientResult<GuildApplicationCommandPermission> {
    //     self.put(
    //         EditApplicationCommandPermissions(application, guild, command),
    //         GuildCommandPermissions {
    //             // id: command,
    //             permissions,
    //         },
    //     ).await
    // }

    // /// Edits command permissions for a specific command for your application in a guild.
    // ///
    // /// This endpoint will overwrite all existing permissions for all commands in a guild.
    // ///
    // /// # Errors
    // ///
    // /// If the http request fails
    // pub async fn batch_edit_application_command_permissions(
    //     &self,
    //     application: ApplicationId,
    //     guild: GuildId,
    //     permissions: Vec<GuildCommandPermissions>,
    // ) -> ClientResult<()> {
    //     self.put_unit(BatchEditApplicationCommandPermissions(application, guild), permissions).await
    // }

    /// Create a response to an Interaction from the gateway.
    ///
    /// # Errors
    ///
    /// If the http request fails
    pub async fn create_interaction_response(
        &self,
        interaction: InteractionId,
        token: Token,
        response: InteractionResponse,
    ) -> ClientResult<InteractionResponse> {
        // match response {
        //     InteractionResponse::Pong
        //     | InteractionResponse::DeferredChannelMessageWithSource
        //     | InteractionResponse::DeferredUpdateMessage => {
        //         self.post_unit(
        //             CreateInteractionResponse(interaction, token),
        //             &response,
        //         ).await.map(|()| response)
        //     }
        //     InteractionResponse::ChannelMessageWithSource(message)
        //     | InteractionResponse::UpdateMessage(message) => {
        //
        //     }
        // }
        self.send_message_with_files(
            CreateInteractionResponse(interaction, token),
            response.clone(),
        ).await.map(|()| response)
        // // todo here (or elsewhere ig) validate InteractionResponse!!!
        // //  thats so good because then it can just ? instead of asserting!
        // //  todo do that ^ everywhere? maybe not since then it gets more separated from why/where
        // //   although it kinda already is iirc
        // //   wtf am I talking about here I'm confused
        // self.post_unit(
        //     CreateInteractionResponse(interaction, token),
        //     &response,
        // ).await.map(|()| response)
    }

    // todo link to EditWebhookMessage?
    /// Edits the initial Interaction response. Functions the same as Edit Webhook Message.
    ///
    /// # Errors
    ///
    /// If the http request fails
    pub async fn edit_interaction_response(
        &self,
        application: ApplicationId,
        token: Token,
        message: InteractionMessage,
    ) -> ClientResult<Message> {
        self.patch(
            EditInteractionResponse(application, token),
            &message,
        ).await
    }

    /// Deletes the initial Interaction response.
    ///
    /// # Errors
    ///
    /// If the http request fails
    pub async fn delete_interaction_response(
        &self,
        application: ApplicationId,
        token: Token,
    ) -> ClientResult<()> {
        self.delete(DeleteInteractionResponse(application, token)).await
    }

    // todo link
    /// Create a followup message for an Interaction. Functions the same as Execute Webhook
    ///
    /// # Errors
    ///
    /// If the http request fails, or fails to deserialize the response into a `Message`
    pub async fn create_followup_message(
        &self,
        application: ApplicationId,
        token: Token,
        message: WebhookMessage,
    ) -> ClientResult<Message> {
        self.send_message_with_files(CreateFollowupMessage(application, token), message).await
    }

    // todo link
    /// Edits a followup message for an Interaction. Functions the same as Edit Webhook Message.
    ///
    /// # Errors
    ///
    /// If the http request fails
    pub async fn edit_followup_message(
        &self,
        application: ApplicationId,
        token: Token,
        message: MessageId,
        edit: WebhookMessage,
    ) -> ClientResult<Message> {
        self.patch(
            EditFollowupMessage(application, token, message),
            &edit,
        ).await
    }

    /// Deletes a followup message for an Interaction.
    ///
    /// # Errors
    ///
    /// If the http request fails
    pub async fn delete_followup_message(
        &self,
        application: ApplicationId,
        token: Token,
        message: MessageId,
    ) -> ClientResult<()> {
        self.delete(DeleteFollowupMessage(application, token, message)).await
    }
}

#[derive(Serialize)]
struct Edit<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<Vec<new_command::CommandOption>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    default_permission: Option<bool>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct WebhookMessage {
    /// the message contents (up to 2000 characters)
    pub content: Cow<'static, str>,
    /// override the default username of the webhook
    pub username: Option<Cow<'static, str>>,
    /// override the default avatar of the webhook
    pub avatar_url: Option<Cow<'static, str>>,
    /// true if this is a TTS message
    pub tts: bool,
    /// the contents of the file being sent
    #[serde(skip)]
    pub files: HashSet<MessageAttachment>,
    /// embedded rich content, up to 10
    pub embeds: Vec<RichEmbed>,
    /// allowed mentions for the message
    pub allowed_mentions: Option<AllowedMentions>,
    /// only [MessageFlags::EPHEMERAL] are allowed
    #[serde(skip_serializing_if = "MessageFlags::is_empty")]
    pub flags: MessageFlags,
    /// the components to include with the message
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<ActionRow>,
}

pub fn webhook_message<F: FnOnce(&mut WebhookMessage)>(builder: F) -> WebhookMessage {
    WebhookMessage::build(builder)
}

impl<S: Into<Cow<'static, str>>> From<S> for WebhookMessage {
    fn from(s: S) -> Self {
        webhook_message(|m| m.content(s))
    }
}

impl From<RichEmbed> for WebhookMessage {
    fn from(e: RichEmbed) -> Self {
        webhook_message(|m| m.embeds = vec![e])
    }
}

impl From<InteractionMessage> for WebhookMessage {
    fn from(InteractionMessage {
                tts,
                content,
                embeds,
                allowed_mentions,
                flags,
                components,
                files
            }: InteractionMessage) -> Self {
        Self {
            content,
            username: None,
            avatar_url: None,
            tts,
            files,
            embeds,
            allowed_mentions,
            flags,
            components,
        }
    }
}

impl WebhookMessage {
    pub fn build<F: FnOnce(&mut Self)>(builder: F) -> Self {
        let mut message = Self::default();
        builder(&mut message);
        message
    }

    pub fn content<S: Into<Cow<'static, str>>>(&mut self, content: S) {
        self.content = content.into();
    }

    pub fn username<S: Into<Cow<'static, str>>>(&mut self, username: S) {
        self.username = Some(username.into());
    }

    pub fn avatar_url<S: Into<Cow<'static, str>>>(&mut self, avatar_url: S) {
        self.avatar_url = Some(avatar_url.into());
    }

    // todo error, don't panic
    /// Add `n` embeds to this [`WebhookMessage`](WebhookMessage), by invoking a builder function
    /// that takes the embed number.
    ///
    /// # Panics
    ///
    /// Panics if adding `n` embeds will result in this [`WebhookMessage`](WebhookMessage) having
    /// more than 10 embeds.
    pub fn embeds<F: FnMut(usize, &mut RichEmbed)>(&mut self, n: usize, mut builder: F) {
        if self.embeds.len() + n > 10 {
            panic!("can't send more than 10 embeds");
        } else {
            self.embeds.extend(
                (0..n).map(|i| embed(|e| builder(i, e)))
            );
        }
    }

    /// add an embed to the [WebhookMessage](WebhookMessage)
    ///
    /// # Panics
    ///
    /// Panics if this message already has 10 or more embeds
    pub fn embed<F: FnOnce(&mut RichEmbed)>(&mut self, builder: F) {
        if self.embeds.len() >= 10 {
            panic!("can't send more than 10 embeds");
        } else {
            self.embeds.push(embed(builder));
        }
    }

    /// add an embed to the [WebhookMessage](WebhookMessage)
    ///
    /// # Errors
    ///
    /// Returns `Err(builder)` if this message already has 10 or more embeds
    pub fn try_embed<F: FnOnce(&mut RichEmbed)>(&mut self, builder: F) -> Result<(), F> {
        if self.embeds.len() >= 10 {
            Err(builder)
        } else {
            self.embeds.push(embed(builder));
            Ok(())
        }
    }

    /// Attach an image to this message. See [`MessageAttachment`] for details about what types impl
    /// `Into<MessageAttachment>`.
    pub fn attachment<A: Into<MessageAttachment>>(&mut self, attachment: A) {
        self.files.insert(attachment.into());
    }

    pub fn ephemeral(&mut self) {
        self.flags.set(MessageFlags::EPHEMERAL, true);
    }

    pub fn button<B, State, C, F>(&mut self, state: State, command: C, builder: F)
        where B: Send + Sync + 'static,
              State: AsRef<BotState<B>>,
              C: ButtonCommand<Bot=B>,
              F: FnOnce(&mut Button),
    {
        let mut button = Button::new();
        builder(&mut button);
        state.as_ref().register_button(&mut button, Box::new(command));
        self.components.push(ActionRow::buttons(vec![button]))
        // self.buttons(iter::once(button))
    }

    pub fn buttons<B, State, I>(&mut self, state: State, buttons: I)
        where B: Send + Sync + 'static,
              State: AsRef<BotState<B>>,
              I: IntoIterator<Item=(Box<dyn ButtonCommand<Bot=B>>, Button)>,
    {
        let buttons = buttons.into_iter()
            .map(|(command, mut button)| {
                state.as_ref().register_button(&mut button, command);
                button
            })
            .collect();
        self.components.push(ActionRow::buttons(buttons))
    }

    pub fn menu<B, State, C, F, D>(&mut self, state: State, command: C, builder: F)
        where B: Send + Sync + 'static,
              State: AsRef<BotState<B>>,
              C: MenuCommand<Bot=B, Data=D>,
              D: MenuData,
              <D as FromStr>::Err: Debug,
              Component: From<Menu<D::Data>>,
              F: FnOnce(&mut Menu<D::Data>),
    {
        let mut menu = Menu::<D::Data>::new();
        menu.options = D::options();
        builder(&mut menu);
        state.as_ref().register_menu(&mut menu, Box::new(command));
        self.components.push(ActionRow::menu(menu))
    }
}
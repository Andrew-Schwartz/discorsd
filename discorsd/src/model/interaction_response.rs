use std::borrow::Cow;
use std::collections::HashSet;

use serde::Serialize;

use crate::BotState;
use crate::commands::{ButtonCommand, MenuCommand};
use crate::http::channel::{embed, MessageAttachment, RichEmbed};
use crate::model::components::{ActionRow, Component, ComponentId};
use crate::model::message::{AllowedMentions, MessageFlags};
use crate::model::new_command::Choice;
use crate::serde_utils::BoolExt;

serde_num_tag! { just Serialize =>
    /// After receiving an interaction, you must respond to acknowledge it. This may be a `pong` for a
    /// `ping`, a message, or simply an acknowledgement that you have received it and will handle the
    /// command async.
    ///
    /// Interaction responses may choose to "eat" the user's command input if you do not wish to have
    /// their slash command show up as message in chat. This may be helpful for slash commands, or
    /// commands whose responses are asynchronous or ephemeral messages.
    #[derive(Debug, Clone)]
    // todo make it be `"type": u8 as InteractionResponseType`
    //  and then it generates InteractionResponseType as a `serde_repr`ed thing 
    pub enum InteractionResponse = "type": u8 {
        /// ACK a `Ping`
        (1) = Pong,
        /// respond to an interaction with a message
        (4) = ChannelMessageWithSource(InteractionMessage),
        /// ACK an interaction and edit a response later, the user sees a loading state
        (5) = DeferredChannelMessageWithSource,
        /// for components ONLY, ACK an interaction and edit the original message later; the user
        /// does not see a loading state
        (6) = DeferredUpdateMessage,
        /// for components ONLY, edit the message the component was attached to
        (7) = UpdateMessage(InteractionMessage),
        /// respond to an autocomplete interaction with suggested choices
        // todo
        (8) = ApplicationCommandAutocompleteResult(Autocomplete),
        /// respond to an interaction with a popup modal
        /// Not available for `MODAL_SUBMIT` and `PING` interactions
        // todo
        (9) = Modal(Modal),
    }
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct InteractionMessage {
    /// is the response TTS
    #[serde(skip_serializing_if = "bool::is_false")]
    pub tts: bool,
    /// message content
    #[serde(skip_serializing_if = "str::is_empty")]
    pub content: Cow<'static, str>,
    /// supports up to 10 embeds
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub embeds: Vec<RichEmbed>,
    /// allowed mentions object
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_mentions: Option<AllowedMentions>,
    /// only [MessageFlags::EPHEMERAL] and [MessageFlags::SUPPRESS_EMBEDS] are allowed
    #[serde(skip_serializing_if = "MessageFlags::is_empty")]
    pub flags: MessageFlags,
    /// message components
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub components: Vec<ActionRow>,
    /// partial attachment objects with filename and description
    #[serde(skip_serializing)]
    pub files: HashSet<MessageAttachment>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Autocomplete {
    /// autocomplete choices (max of 25 choices)
    // todo the type of the choice has to be generic lol
    pub choices: Vec<Choice<()>>,
}

#[derive(Serialize, Debug, Clone)]
pub struct Modal {
    /// a developer-defined identifier for the modal, max 100 characters
    pub custom_id: ComponentId,
    /// the title of the popup modal, max 45 characters
    pub title: String,
    /// between 1 and 5 (inclusive) components that make up the modal
    pub components: Vec<Component>,
}

pub fn message<F: FnOnce(&mut InteractionMessage)>(builder: F) -> InteractionMessage {
    InteractionMessage::build(builder)
}

pub fn ephemeral<C: Into<Cow<'static, str>>>(content: C) -> InteractionMessage {
    message(|m| {
        m.content(content);
        m.ephemeral();
    })
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

    pub fn button<B, Btn>(&mut self, state: &BotState<B>, button: Btn)
        where B: Send + Sync + 'static,
              Btn: ButtonCommand<Bot=B> + 'static,
    {
        self.buttons(state, [Box::new(button) as _])
    }

    pub fn buttons<B, I>(&mut self, state: &BotState<B>, buttons: I)
        where B: Send + Sync + 'static,
              I: IntoIterator<Item=Box<dyn ButtonCommand<Bot=B>>>,
    {
        let mut component_buttons = Vec::new();
        for button in buttons {
            component_buttons.push(state.make_button(button));
        }
        self.components.push(ActionRow::buttons(component_buttons));
    }

    pub fn menu<B, M>(&mut self, state: &BotState<B>, menu: M)
        where B: Send + Sync + 'static,
              M: MenuCommand<Bot=B> + 'static,
    {
        let menu = state.make_string_menu(Box::new(menu));
        self.components.push(ActionRow::select_menu(menu))
    }
}
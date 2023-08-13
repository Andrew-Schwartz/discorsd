use std::borrow::Cow;
use std::collections::HashSet;
use std::fmt::Debug;
use std::str::FromStr;

use serde::Serialize;

use crate::BotState;
use crate::commands::{ButtonCommand, MenuCommand, MenuData, ModalCommand};
use crate::http::channel::{embed, MessageAttachment, RichEmbed};
use crate::model::components::{ActionRow, Button, Component, ComponentId, make_button, make_text_input, Menu, TextInput, TextInputStyle};
use crate::model::message::{AllowedMentions, MessageFlags};
use crate::model::command::Choice;
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
    pub enum InteractionResponse = "type": u8, inner = "data" {
        /// ACK a `Ping`
        (1) = Pong,
        /// respond to an interaction with a message
        (4) = ChannelMessageWithSource(InteractionMessage),
        /// ACK an interaction and edit a response later, thse user sees a loading state
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
        (9) = Modal(Modal),
    }
}

#[test]
fn serialize_interaction_response() {
    let response = InteractionResponse::ChannelMessageWithSource(InteractionMessage {
        tts: false,
        content: "MyContent".into(),
        embeds: vec![],
        allowed_mentions: None,
        flags: Default::default(),
        components: vec![],
        files: Default::default(),
    });
    let json = serde_json::to_string_pretty(&response).unwrap();
    // todo has to be {"type": 1, "data": {"content":...,...}}
    println!("{json}");
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

#[derive(Serialize, Debug, Clone, Default)]
pub struct Autocomplete {
    /// autocomplete choices (max of 25 choices)
    // todo the type of the choice has to be generic lol
    pub choices: Vec<Choice<()>>,
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct Modal {
    /// a developer-defined identifier for the modal, max 100 characters
    pub custom_id: ComponentId,
    /// the title of the popup modal, max 45 characters
    #[serde(skip_serializing_if = "str::is_empty")]
    pub title: Cow<'static, str>,
    /// between 1 and 5 (inclusive) components that make up the modal
    pub components: Vec<ActionRow>,
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
            .unwrap();
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

    /// Attach an image to this message. See [`MessageAttachment`] for details about what types impl
    /// `Into<MessageAttachment>`.
    pub fn attach<A: Into<MessageAttachment>>(&mut self, attachment: A) {
        self.files.insert(attachment.into());
    }

    pub fn ephemeral(&mut self) {
        self.flags.set(MessageFlags::EPHEMERAL, true);
    }

    pub fn button<B, State, C, F>(&mut self, state: State, command: C, builder: F)
        where B: 'static,
              State: AsRef<BotState<B>>,
              C: ButtonCommand<Bot=B>,
              F: FnOnce(&mut Button),
    {
        let mut button = make_button(builder);
        state.as_ref().register_button(&mut button, Box::new(command));
        self.components.push(ActionRow::buttons(vec![button]));
        // self.buttons(iter::once(button))
    }

    pub fn buttons<B, State, I>(&mut self, state: State, buttons: I)
        where B: 'static,
              State: AsRef<BotState<B>>,
              I: IntoIterator<Item=(Box<dyn ButtonCommand<Bot=B>>, Button)>,
    {
        let buttons = buttons.into_iter()
            .map(|(command, mut button)| {
                state.as_ref().register_button(&mut button, command);
                button
            })
            .collect();
        self.components.push(ActionRow::buttons(buttons));
    }

    pub fn menu<B, State, C, F, D>(&mut self, state: State, command: C, builder: F)
        where B: 'static,
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
        self.components.push(ActionRow::menu(menu));
    }
}

pub fn auto_complete<F: FnOnce(&mut Autocomplete)>(builder: F) -> Autocomplete {
    Autocomplete::build(builder)
}

// todo
impl Autocomplete {
    // todo avoid code repetition?
    pub fn build_with<F: FnOnce(&mut Self)>(mut with: Self, builder: F) -> Self {
        builder(&mut with);
        with
    }

    pub fn build<F: FnOnce(&mut Self)>(builder: F) -> Self {
        Self::build_with(Self::default(), builder)
    }
}

pub fn modal<B, State, C, F>(state: State, command: C, builder: F) -> Modal
    where B: 'static,
          State: AsRef<BotState<B>>,
          C: ModalCommand<Bot=B>,
          F: FnOnce(&mut Modal),
{
    let mut modal = Modal::build(builder);
    state.as_ref().register_modal(&mut modal, Box::new(command));
    modal
}

// todo test Modal

impl Modal {
    // todo avoid code repetition?
    pub fn build_with<F: FnOnce(&mut Self)>(mut with: Self, builder: F) -> Self {
        builder(&mut with);
        with
    }

    pub fn build<F: FnOnce(&mut Self)>(builder: F) -> Self {
        Self::build_with(Self::default(), builder)
    }

    pub fn title<S: Into<Cow<'static, str>>>(&mut self, title: S) {
        self.title = title.into();
    }

    pub fn text_input<B,State,F>(&mut self, state: State, builder: F)
        where B: 'static,
              State: AsRef<BotState<B>>,
              F: FnOnce(&mut TextInput),
    {
        let mut text_input = make_text_input(builder);
        state.as_ref().register_text_input(&mut text_input);
        self.components.push(ActionRow::text_input(text_input));
    }

    // todo fix or delete?
    /*
    pub fn text_inputs<F,I>(&mut self, text_inputs: I)
        where
            F: FnOnce(&mut TextInput),
            I: IntoIterator,
    {
        let text_inputs: Vec<TextInput> = text_inputs.into();
        text_inputs.iter().map(|ti| {
            self.components.push(ActionRow::text_input(ti));
        });
    }*/
}

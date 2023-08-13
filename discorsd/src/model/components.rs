use std::borrow::Cow;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use crate::BotState;

use crate::model::channel::ChannelType;
use crate::model::emoji::Emoji;
use crate::model::ids::{ChannelId, MentionableId, RoleId, UserId};
use crate::serde_utils::{BoolExt, SkipUnit};

// todo can this be deleted?
/*
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(untagged)]
pub enum Components {
    Inline(Vec<ActionRow>),
    /// currently only the TextInput component is supported
    Modal {
        /// a developer-defined identifier for the modal, max 100 characters
        custom_id: ComponentId,
        /// the title of the popup modal, max 45 characters
        title: String,
        /// between 1 and 5 (inclusive) components that make up the modal
        components: Vec<Component>,
    },
}*/

serde_num_tag! {
    #[derive(Debug, Clone, PartialEq)]
    pub enum ActionRow = "type": ComponentType {
        (ComponentType::ActionRow) = ActionRow {
            components: Vec<Component>,
        }
    }
}

impl ActionRow {
    pub fn new(components: Vec<Component>) -> Self {
        // todo validate at most 5 components
        Self::ActionRow { components }
    }

    pub fn buttons(buttons: Vec<Button>) -> Self {
        Self::new(buttons.into_iter()
            .map(Component::Button)
            .collect())
    }

    pub fn menu<T: SelectMenuType>(menu: Menu<T>) -> Self
        where Component: From<Menu<T>>,
    {
        Self::new(vec![menu.into()])
    }

    pub fn text_input(input: TextInput) -> Self {
        Self::new(vec![Component::TextInput(input)])
    }
}

serde_repr! {
    pub enum ComponentType: u8 {
        ActionRow = 1,
        Button = 2,
        StringMenu = 3,
        TextInput = 4,
        UserMenu = 5,
        RoleMenu = 6,
        MentionableMenu = 7,
        ChannelMenu = 8,
    }
}

serde_num_tag! {
    #[derive(Debug, Clone, PartialEq)]
    pub enum Component = "type": ComponentType {
        // (ComponentType::ActionRow) = ActionRow(ActionRow), // todo add? (for modal)
        (ComponentType::Button) = Button(Button),
        (ComponentType::StringMenu) = SelectString(Menu<String>),
        (ComponentType::TextInput) = TextInput(TextInput),
        (ComponentType::UserMenu) = SelectUser(Menu<UserId>),
        (ComponentType::RoleMenu) = SelectRole(Menu<RoleId>),
        (ComponentType::MentionableMenu) = SelectMentionable(Menu<MentionableId>),
        (ComponentType::ChannelMenu) = SelectChannel(Menu<ChannelId>),
    }
}
macro_rules! from_menu {
    ($($t:ty => $var:ident);* $(;)?) => {
        $(
            impl From<Menu<$t>> for Component {
                fn from(value: Menu<$t>) -> Self {
                    Self::$var(value)
                }
            }
        )+
    };
}
from_menu! {
    String => SelectString;
    UserId => SelectUser;
    RoleId => SelectRole;
    MentionableId => SelectMentionable;
    ChannelId => SelectChannel;
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash, Default)]
#[serde(transparent)]
pub struct ComponentId(pub(crate) String);

impl<S> From<S> for ComponentId
    where String: From<S> {
    fn from(s: S) -> Self {
        Self(String::from(s))
    }
}

// todo have custom_id and url as an enum of some sort
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct Button {
    /// one of button styles
    style: ButtonStyle,
    /// text that appears on the button, max 80 characters
    #[serde(default, skip_serializing_if = "Option::is_none")]
    label: Option<Cow<'static, str>>,
    // todo just those fields not the whole emoji
    /// emoji name, id, and animated,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    emoji: Option<Emoji>,
    /// a developer-defined identifier for the component, max 100 characters
    ///
    /// required unless url is set
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) custom_id: Option<ComponentId>,
    /// a url for link-style buttons,
    ///
    /// required unless custom_id is set
    #[serde(default, skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    /// whether the component is disabled, default false
    #[serde(default, skip_serializing_if = "bool::is_false")]
    disabled: bool,
}

impl Button {
    pub(crate) fn new() -> Self {
        Self {
            style: ButtonStyle::Primary,
            label: None,
            emoji: None,
            custom_id: None,
            url: None,
            disabled: false,
        }
    }

    /// text that appears on the button, max 80 characters
    pub fn label<S: Into<Cow<'static, str>>>(&mut self, label: S) {
        self.label = Some(label.into());
    }

    /// one of button styles
    pub fn style(&mut self, style: ButtonStyle) {
        self.style = style;
    }

    /// whether the button is disabled, default false
    pub fn disable(&mut self) {
        self.disabled = true;
    }
}

pub fn make_button<F: FnOnce(&mut Button)>(builder: F) -> Button {
    let mut button = Button::new();
    builder(&mut button);
    button
}

/*
(\w+)\t\d\t([\w, ]+)\t(\w+)

\t\t/// Color: $3. Required Field: `$4`.
\t\t$1 = $2,
 */

serde_repr! {
    /// <https://discord.com/developers/docs/interactions/message-components#button-object-button-styles>
    pub enum ButtonStyle: u8 {
		/// Color: blurple. Required Field: `custom_id`.
		Primary = 1,
		/// Color: grey. Required Field: `custom_id`.
		Secondary = 2,
		/// Color: green. Required Field: `custom_id`.
		Success = 3,
		/// Color: red. Required Field: `custom_id`.
		Danger = 4,
		/// Color: grey, navigates to a URL. Required Field: `url`.
		Link = 5,
    }
}

pub trait SelectMenuType {
    type SelectOption: Serialize + for<'a> Deserialize<'a>;
    type ChannelTypes: Default + SkipUnit;
}
macro_rules! smt {
    ($($t:ty => $so:ty, $ct:ty);* $(;)?) => {
        $(
            impl SelectMenuType for $t {
                type SelectOption = $so;
                type ChannelTypes = $ct;
            }
        )*
    };
}
smt! {
    String => SelectOption, ();
    UserId => (), ();
    RoleId => (), ();
    MentionableId => (), ();
    ChannelId => (), Vec<ChannelType>;
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Menu<T: SelectMenuType> {
    /// a developer-defined identifier for the button, max 100 characters
    pub(crate) custom_id: ComponentId,
    /// the choices in the select, max 25
    #[serde(default, skip_serializing_if = "SkipUnit::should_skip")]
    pub(crate) options: Vec<T::SelectOption>,
    /// List of channel types to include
    #[serde(default, skip_serializing_if = "SkipUnit::should_skip")]
    channel_types: T::ChannelTypes,
    /// custom placeholder text if nothing is selected, max 100 characters
    #[serde(default, skip_serializing_if = "Option::is_none")]
    placeholder: Option<Cow<'static, str>>,
    /// the minimum number of items that must be chosen; default 1, min 0, max 25
    #[serde(default, skip_serializing_if = "Option::is_none")]
    min_values: Option<usize>,
    /// the maximum number of items that can be chosen; default 1, max 25
    #[serde(default, skip_serializing_if = "Option::is_none")]
    max_values: Option<usize>,
    /// disable the select menu, default false
    #[serde(default, skip_serializing_if = "bool::is_false")]
    disabled: bool,
}

impl<T: SelectMenuType> Menu<T> {
    pub(crate) fn new() -> Self {
        Self {
            custom_id: ComponentId(String::new()),
            options: Vec::new(),
            channel_types: T::ChannelTypes::default(),
            placeholder: None,
            min_values: None,
            max_values: None,
            disabled: false,
        }
    }

    /// custom placeholder text if nothing is selected, max 100 characters
    pub fn placeholder<S: Into<Cow<'static, str>>>(&mut self, placeholder: S) {
        self.placeholder = Some(placeholder.into());
    }

    /// the minimum number of items that must be chosen; default 1, min 0, max 25
    pub fn min_values(&mut self, min: usize) {
        self.min_values = Some(min);
    }

    /// the maximum number of items that can be chosen; default 1, max 25
    pub fn max_values(&mut self, max: usize) {
        self.max_values = Some(max);
    }

    /// the minimum (default 1, min 0, max 25) and maximum (default 1, max 25) number of items that
    /// can be chosen
    pub fn min_max_values(&mut self, min: usize, max: usize) {
        self.min_values(min);
        self.max_values(max);
    }

    /// disable the select menu, default false
    pub fn disable(&mut self) {
        self.disabled = true;
    }
}

impl Menu<String> {
    /// the choices in the select, max 25
    pub fn options(&mut self, options: Vec<SelectOption>) {
        self.options = options.into_iter().unique_by(|o| o.value.clone()).collect();
    }

    pub fn default_options<F: Fn(&str) -> bool>(&mut self, is_default: F) {
        self.options.iter_mut()
            .filter(|opt| is_default(&opt.value))
            .for_each(|opt| opt.default = true);
    }
}

impl Menu<ChannelId> {
    /// list of channel types to include
    pub fn channel_types(&mut self, channel_types: Vec<ChannelType>) {
        self.channel_types = channel_types;
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct SelectOption {
    /// the user-facing name of the option, max 100 characters
    pub label: String,
    /// the dev-define value of the option, max 100 characters
    pub value: String,
    /// an additional description of the option, max 100 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// id, name, and animated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<Emoji>,
    /// will render this option as selected by default
    #[serde(default, skip_serializing_if = "bool::is_false")]
    pub default: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq, Eq)]
pub struct TextInput {
    /// Developer-defined identifier for the input; max 100 characters
    pub(crate) custom_id: ComponentId,
    /// The Text Input Style
    pub style: TextInputStyle,
    /// Label for this component; max 45 characters
    pub label: Cow<'static, str>,
    /// Minimum input length for a text input; min 0, max 4000
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_length: Option<usize>,
    /// Maximum input length for a text input; min 1, max 4000
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_length: Option<usize>,
    /// Whether this component is required to be filled (defaults to true)
    #[serde(default = "bool::default_true", skip_serializing_if = "bool::is_true")]
    pub required: bool,
    /// Pre-filled value for this component; max 4000 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    // todo change placeholder to use Cow like Menu?
    /// Custom placeholder text if the input is empty; max 100 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
}

serde_repr! {
    pub enum TextInputStyle: u8 {
        Short = 1,
        Paragraph = 2,
    }
}

// todo add more fns?
impl TextInput {
    pub(crate) fn new() -> Self {
        Self {
            custom_id: ComponentId(String::new()),
            style: TextInputStyle::Short,
            label: Cow::from(String::new()),
            min_length: None,
            max_length: None,
            required: false,
            value: None,
            placeholder: None,
        }
    }

    /// text that appears over the text input, max 45 characters
    pub fn label<S: Into<Cow<'static, str>>>(&mut self, label: S) {
        self.label = label.into();
    }

    /// one of button styles
    pub fn style(&mut self, style: TextInputStyle) {
        self.style = style;
    }
}

pub fn make_text_input<F: FnOnce(&mut TextInput)>(builder: F) -> TextInput {
    let mut text_input = TextInput::new();
    builder(&mut text_input);
    text_input
}

// Testing:

#[cfg(test)]
mod tests {
    use crate::model::emoji::CustomEmoji;
    use crate::model::ids::EmojiId;

    use super::*;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct MyMessage {
        content: &'static str,
        components: Vec<ActionRow>,
    }

    fn test(correct: &'static str, message: MyMessage) {
        let serialized = serde_json::to_string_pretty(&message).unwrap();
        assert_eq!(serialized, correct);
        let deserialized: MyMessage = serde_json::from_str(correct).unwrap();
        assert_eq!(deserialized, message);
    }

    #[test]
    fn empty_component() {
        const CORRECT: &str = r#"{
  "content": "This is a message with components",
  "components": [
    {
      "type": 1,
      "components": []
    }
  ]
}"#;
        let message = MyMessage {
            content: "This is a message with components",
            components: vec![
                ActionRow::new(vec![])
            ],
        };
        test(CORRECT, message);
    }

    #[test]
    fn action_row_with_button() {
        const CORRECT: &str = r#"{
  "content": "This is a message with components",
  "components": [
    {
      "type": 1,
      "components": [
        {
          "type": 2,
          "style": 1,
          "label": "Click me!",
          "custom_id": "click_one"
        }
      ]
    }
  ]
}"#;
        let message = MyMessage {
            content: "This is a message with components",
            components: vec![
                ActionRow::new(vec![Component::Button(
                    Button {
                        style: ButtonStyle::Primary,
                        label: Some("Click me!".to_string()),
                        emoji: None,
                        custom_id: Some("click_one".into()),
                        url: None,
                        disabled: false,
                    })
                ])
            ],
        };
        test(CORRECT, message);
        let message = MyMessage {
            content: "This is a message with components",
            components: vec![
                ActionRow::buttons(vec![
                    Button {
                        style: ButtonStyle::Primary,
                        label: Some("Click me!".into()),
                        emoji: None,
                        custom_id: Some("click_one".into()),
                        url: None,
                        disabled: false,
                    }
                ])
            ],
        };
        test(CORRECT, message)
    }

    #[test]
    fn action_row_with_menu() {
        const CORRECT: &str = r#"{
  "content": "Mason is looking for new arena partners. What classes do you play?",
  "components": [
    {
      "type": 1,
      "components": [
        {
          "type": 3,
          "custom_id": "class_select_1",
          "options": [
            {
              "label": "Rogue",
              "value": "rogue",
              "description": "Sneak n stab",
              "emoji": {
                "id": "625891304148303894",
                "name": "rogue"
              }
            },
            {
              "label": "Mage",
              "value": "mage",
              "description": "Turn 'em into a sheep",
              "emoji": {
                "id": "625891304081063986",
                "name": "mage"
              }
            },
            {
              "label": "Priest",
              "value": "priest",
              "description": "You get heals when I'm done doing damage",
              "emoji": {
                "id": "625891303795982337",
                "name": "priest"
              }
            }
          ],
          "placeholder": "Choose a class",
          "min_values": 1,
          "max_values": 3
        }
      ]
    }
  ]
}"#;
        let message = MyMessage {
            content: "Mason is looking for new arena partners. What classes do you play?",
            components: vec![ActionRow::menu::<String>(Menu {
                custom_id: "class_select_1".into(),
                options: vec![
                    SelectOption {
                        label: "Rogue".to_string(),
                        value: "rogue".to_string(),
                        description: Some("Sneak n stab".to_string()),
                        emoji: Some(Emoji::Custom(CustomEmoji::new(
                            EmojiId(625891304148303894),
                            "rogue",
                        ))),
                        default: false,
                    }, SelectOption {
                        label: "Mage".to_string(),
                        value: "mage".to_string(),
                        description: Some("Turn 'em into a sheep".to_string()),
                        emoji: Some(Emoji::Custom(CustomEmoji::new(
                            EmojiId(625891304081063986),
                            "mage",
                        ))),
                        default: false,
                    },
                    SelectOption {
                        label: "Priest".to_string(),
                        value: "priest".to_string(),
                        description: Some("You get heals when I'm done doing damage".to_string()),
                        emoji: Some(Emoji::Custom(CustomEmoji::new(
                            EmojiId(625891303795982337),
                            "priest",
                        ))),
                        default: false,
                    },
                ],
                channel_types: (),
                placeholder: Some("Choose a class".to_string()),
                min_values: Some(1),
                max_values: Some(3),
                disabled: false,
            })],
        };
        test(CORRECT, message);
    }

    #[test]
    fn action_row_with_text_input() {
        const CORRECT: &str = r#"{
  "content": "Message content",
  "components": {
    "title": "My Cool Modal",
    "custom_id": "cool_modal",
    "components": [{
      "type": 1,
      "components": [{
        "type": 4,
        "custom_id": "name",
        "label": "Name",
        "style": 1,
        "min_length": 1,
        "max_length": 4000,
        "placeholder": "John",
        "required": true
      }]
    }]
  }
}"#;
        // let message = Message {
        //     content: "Message content",
        //     components: vec![
        //         ActionRow::text_input(TextInput {
        //             custom_id: (),
        //             style: TextInputStyle::Short,
        //             label: (),
        //             min_length: None,
        //             max_length: None,
        //             required: false,
        //             value: None,
        //             placeholder: None,
        //         })
        //     ],
        // };
        // todo
    }
}
use serde::{Deserialize, Serialize};

use crate::model::channel::ChannelType;
use crate::model::emoji::Emoji;
use crate::model::ids::{ChannelId, UserId, RoleId, MentionableId};
use crate::serde_utils::{BoolExt, SkipUnit};

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
        components: Vec<Component>
    }
}

serde_num_tag! {
    #[derive(Debug, Clone, PartialEq)]
    pub enum ActionRow = "type": ComponentType {
        (ComponentType::ActionRow) = ActionRow {
            components: Vec<Component>,
        }
    }
}

// #[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
// pub struct ActionRow {
//     #[serde(rename = "type")]
//     kind: u8,
//     components: Vec<Component>,
// }

impl ActionRow {
    pub fn action_row(components: Vec<Component>) -> Self {
        // todo validate at most 5 components
        Self::ActionRow { components }
    }

    pub fn buttons(buttons: Vec<Button>) -> Self {
        Self::action_row(buttons.into_iter()
            .map(Component::Button)
            .collect())
    }

    pub fn select_menu(menu: SelectMenu<String>) -> Self {
        Self::action_row(vec![Component::SelectString(menu)])
    }

    pub fn text_input(input: TextInput) -> Self {
        Self::action_row(vec![Component::TextInput(input)])
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

// todo is this just serialize?
serde_num_tag! {
    #[derive(Debug, Clone, PartialEq)]
    pub enum Component = "type": ComponentType {
        (ComponentType::Button) = Button(Button),
        (ComponentType::StringMenu) = SelectString(SelectMenu<String>),
        (ComponentType::TextInput) = TextInput(TextInput),
        (ComponentType::UserMenu) = SelectUser(SelectMenu<UserId>),
        (ComponentType::RoleMenu) = SelectRole(SelectMenu<RoleId>),
        (ComponentType::MentionableMenu) = SelectMentionable(SelectMenu<MentionableId>),
        (ComponentType::ChannelMenu) = SelectChannel(SelectMenu<ChannelId>),
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(transparent)]
// todo should maybe be Arc??
pub struct ComponentId(String);

impl<S> From<S> for ComponentId
    where String: From<S> {
    fn from(s: S) -> Self {
        Self(String::from(s))
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct Button {
    /// one of button styles
    pub style: ButtonStyle,
    /// text that appears on the button, max 80 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// emoji name, id, and animated,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<Emoji>,
    /// a developer-defined identifier for the component, max 100 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_id: Option<ComponentId>,
    /// a url for link-style buttons,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// whether the component is disabled, default false
    #[serde(skip_serializing_if = "bool::is_false")]
    pub disabled: bool,
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
    type SelectOptions: SkipUnit;
    type ChannelTypes: SkipUnit;
}
macro_rules! smt {
    ($($t:ty => $so:ty, $ct:ty);* $(;)?) => {
        $(
            impl SelectMenuType for $t {
                type SelectOptions = $so;
                type ChannelTypes = $ct;
            }
        )*
    };
}
smt! {
    String => Vec<SelectOption>, ();
    UserId => (), ();
    RoleId => (), ();
    MentionableId => (), ();
    ChannelId => (), Vec<ChannelType>;
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct SelectMenu<T: SelectMenuType> {
    /// a developer-defined identifier for the button, max 100 characters
    pub custom_id: ComponentId,
    /// the choices in the select, max 25
    #[serde(default, skip_serializing_if = "SkipUnit::should_skip")]
    pub options: T::SelectOptions,
    /// List of channel types to include
    #[serde(default, skip_serializing_if = "SkipUnit::should_skip")]
    pub channel_types: T::ChannelTypes,
    /// custom placeholder text if nothing is selected, max 100 characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    /// the minimum number of items that must be chosen; default 1, min 0, max 25
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_values: Option<u8>,
    /// the maximum number of items that can be chosen; default 1, max 25
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_values: Option<u8>,
    /// disable the select, default false
    #[serde(skip_serializing_if = "bool::is_false")]
    pub disabled: bool,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct TextInput {
    /// Developer-defined identifier for the input; max 100 characters
	pub custom_id: String,
    /// The Text Input Style
	pub style: TextInputStyle,
    /// Label for this component; max 45 characters
	pub label: String,
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
                ActionRow::action_row(vec![])
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
                ActionRow::action_row(vec![Component::Button(
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
            components: vec![ActionRow::select_menu(SelectMenu {
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
use serde::{Serialize, Deserialize, Serializer};
use crate::model::emoji::Emoji;
use crate::serde_utils::BoolExt;
use serde::de::Error;
use std::convert::TryFrom;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct ActionRow {
    #[serde(rename = "type")]
    kind: u8,
    components: Vec<Component>,
}

impl ActionRow {
    pub fn action_row(components: Vec<Component>) -> Self {
        // todo validate at most 5 components
        Self { kind: 1, components }
    }

    pub fn buttons(buttons: Vec<Button>) -> Self {
        Self::action_row(buttons.into_iter()
            .map(Component::Button)
            .collect())
    }

    pub fn select_menu(menu: SelectMenu) -> Self {
        Self::action_row(vec![Component::SelectMenu(menu)])
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(try_from = "RawComponent")]
pub enum Component {
    Button(Button),
    SelectMenu(SelectMenu),
}

impl Serialize for Component {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct Shim<'a, R> {
            #[serde(flatten)]
            row: &'a R,
            #[serde(rename = "type")]
            t: u8,
        }

        match self {
            Self::Button(buttons) => Shim { row: buttons, t: 2 }.serialize(s),
            Self::SelectMenu(menu) => Shim { row: menu, t: 3 }.serialize(s),
        }
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

#[derive(Deserialize)]
struct RawComponent {
    #[serde(rename = "type")]
    kind: u8,
    custom_id: Option<ComponentId>,
    disabled: Option<bool>,
    style: Option<ButtonStyle>,
    label: Option<String>,
    emoji: Option<Emoji>,
    url: Option<String>,
    options: Option<Vec<SelectOption>>,
    placeholder: Option<String>,
    min_values: Option<u8>,
    max_values: Option<u8>,
    // components: Option<Vec<RawComponent>>,
}

impl TryFrom<RawComponent> for Component {
    type Error = crate::serde_utils::Error;

    fn try_from(raw: RawComponent) -> Result<Self, Self::Error> {
        // todo this
        match raw.kind {
            1 => Err(Self::Error::Serde(serde_json::Error::custom("todo not ActionRow"))),
            2 => Ok(Self::Button(Button {
                style: raw.style.unwrap(),
                label: raw.label,
                emoji: raw.emoji,
                custom_id: raw.custom_id,
                url: raw.url,
                disabled: raw.disabled.unwrap_or(false),
            })),
            3 => Ok(Self::SelectMenu(SelectMenu {
                custom_id: raw.custom_id.unwrap(),
                options: raw.options.unwrap(),
                placeholder: raw.placeholder,
                min_values: raw.min_values,
                max_values: raw.max_values,
                disabled: raw.disabled.unwrap_or(false),
            })),
            _bad => todo!(),
        }
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

/*
(\w+)\??\t(.+)\t(.+)

/// $3
\tpub $1: Option<$2>,
 */

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct SelectMenu {
    /// a developer-defined identifier for the button, max 100 characters
    pub custom_id: ComponentId,
    /// the choices in the select, max 25
    pub options: Vec<SelectOption>,
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

// Testing:

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::emoji::CustomEmoji;
    use crate::model::ids::EmojiId;

    #[derive(Serialize, Deserialize, Debug)]
    struct Message {
        content: &'static str,
        components: Vec<ActionRow>,
    }

    // todo?
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
        let message = Message {
            content: "This is a message with components",
            components: vec![
                ActionRow::action_row(vec![])
            ],
        };
        let serialized = serde_json::to_string_pretty(&message).unwrap();
        println!("serialized = {:#?}", serialized);
        assert_eq!(serialized, CORRECT);
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
          "style": 1,
          "label": "Click me!",
          "custom_id": "click_one",
          "type": 2
        }
      ]
    }
  ]
}"#;
        let message = Message {
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
        let serialized = serde_json::to_string_pretty(&message).unwrap();
        println!("serialized = {:#?}", serialized);
        assert_eq!(serialized, CORRECT);
        let message = Message {
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
        let serialized = serde_json::to_string_pretty(&message).unwrap();
        println!("serialized = {:#?}", serialized);
        assert_eq!(serialized, CORRECT);
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
          "max_values": 3,
          "type": 3
        }
      ]
    }
  ]
}"#;
        let message = Message {
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
                placeholder: Some("Choose a class".to_string()),
                min_values: Some(1),
                max_values: Some(3),
                disabled: false,
            })],
        };
        let serialized = serde_json::to_string_pretty(&message).unwrap();
        println!("serialized = {:#?}", serialized);
        assert_eq!(serialized, CORRECT);
    }
}
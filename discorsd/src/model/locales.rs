use serde_derive::{Serialize, Deserialize};

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash, Serialize, Deserialize)]
pub enum Locale {
	/// da - Dansk
	#[serde(rename="da")]
	Danish,
	/// de - Deutsch
	#[serde(rename="de")]
	German,
	/// en-GB - English, UK
	#[serde(rename="en-GB")]
	EnglishUk,
	/// en-US - English, US
	#[serde(rename="en-US")]
	EnglishUs,
	/// es-ES - Español
	#[serde(rename="es-ES")]
	Spanish,
	/// fr - Français
	#[serde(rename="fr")]
	French,
	/// hr - Hrvatski
	#[serde(rename="hr")]
	Croatian,
	/// it - Italiano
	#[serde(rename="it")]
	Italian,
	/// lt - Lietuviškai
	#[serde(rename="lt")]
	Lithuanian,
	/// hu - Magyar
	#[serde(rename="hu")]
	Hungarian,
	/// nl - Nederlands
	#[serde(rename="nl")]
	Dutch,
	/// no - Norsk
	#[serde(rename="no")]
	Norwegian,
	/// pl - Polski
	#[serde(rename="pl")]
	Polish,
	/// pt-BR - Português do Brasil
	#[serde(rename="pt-BR")]
	PortugueseBrazilian,
	/// ro - Română
	#[serde(rename="ro")]
	Romanian,
	/// fi - Suomi
	#[serde(rename="fi")]
	Finnish,
	/// sv-SE - Svenska
	#[serde(rename="sv-SE")]
	Swedish,
	/// vi - Tiếng Việt
	#[serde(rename="vi")]
	Vietnamese,
	/// tr - Türkçe
	#[serde(rename="tr")]
	Turkish,
	/// cs - Čeština
	#[serde(rename="cs")]
	Czech,
	/// el - Ελληνικά
	#[serde(rename="el")]
	Greek,
	/// bg - български
	#[serde(rename="bg")]
	Bulgarian,
	/// ru - Pусский
	#[serde(rename="ru")]
	Russian,
	/// uk - Українська
	#[serde(rename="uk")]
	Ukrainian,
	/// hi - हिन्दी
	#[serde(rename="hi")]
	Hindi,
	/// th - ไทย
	#[serde(rename="th")]
	Thai,
	/// zh-CN - 中文
	#[serde(rename="zh-CN")]
	ChineseChina,
	/// ja - 日本語
	#[serde(rename="ja")]
	Japanese,
	/// zh-TW - 繁體中文
	#[serde(rename="zh-TW")]
	ChineseTaiwan,
	/// ko - 한국어
	#[serde(rename="ko")]
	Korean,
}
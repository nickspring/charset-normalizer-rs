use crate::entity::Language;
use ahash::{HashMap, HashSet};

use once_cell::sync::Lazy;
use std::iter::FromIterator;

pub(crate) struct LanguageEntry {
    pub language: Language,
    pub alphabet: &'static str,
    pub alphabet_set: HashSet<char>,
    pub have_accents: bool,
    pub pure_latin: bool,
}

impl LanguageEntry {
    pub fn new(
        language: Language,
        alphabet: &'static str,
        have_accents: bool,
        pure_latin: bool,
    ) -> Self {
        Self {
            language,
            alphabet,
            alphabet_set: alphabet.chars().collect(),
            have_accents,
            pure_latin,
        }
    }

    pub fn get(language: &Language) -> Result<&Self, String> {
        for entry in LANGUAGES.iter() {
            if entry.language == *language {
                return Ok(entry);
            }
        }
        Err(String::from("Language wasn't found"))
    }
}

pub(crate) static LANGUAGES: Lazy<Vec<LanguageEntry>> = Lazy::new(|| {
    vec![
    // language, alphabet, have_accents, pure_latin
    LanguageEntry::new(Language::English, "eationsrhldcmufpgwbyvkjxzq", false, true, ),
    LanguageEntry::new(Language::English, "eationsrhldcumfpgwybvkxjzq", false, true, ),
    LanguageEntry::new(Language::German, "enirstadhulgocmbfkwzpvüäöj", true, true, ),
    LanguageEntry::new(Language::French, "easnitrluodcpmévgfbhqàxèyj", true, true, ),
    LanguageEntry::new(Language::Dutch, "enairtodslghvmukcpbwjzfyxë", true, true, ),
    LanguageEntry::new(Language::Italian, "eiaonltrscdupmgvfbzhqèàkyò", true, true, ),
    LanguageEntry::new(Language::Polish, "aioenrzwsctkydpmuljłgbhąęó", true, true, ),
    LanguageEntry::new(Language::Spanish, "eaonsrildtcumpbgvfyóhqíjzá", true, true, ),
    LanguageEntry::new(Language::Russian, "оаеинстрвлкмдпугяызбйьчхжц", false, false, ),
    LanguageEntry::new(Language::Japanese, "人一大亅丁丨竹笑口日今二彳行十土丶寸寺時乙丿乂气気冂巾亠市目儿見八小凵県月彐門間木東山出本中刀分耳又取最言田心思刂前京尹事生厶云会未来白冫楽灬馬尸尺駅明耂者了阝都高卜占厂广店子申奄亻俺上方冖学衣艮食自", false, false, ),
    LanguageEntry::new(Language::Japanese, "ーンス・ルトリイアラックドシレジタフロカテマィグバムプオコデニウメサビナブャエュチキズダパミェョハセベガモツネボソノァヴワポペピケゴギザホゲォヤヒユヨヘゼヌゥゾヶヂヲヅヵヱヰヮヽ゠ヾヷヿヸヹヺ", false, false, ),
    LanguageEntry::new(Language::Japanese, "のにるたとはしいをでてがなれからさっりすあもこまうくよきんめおけそつだやえどわちみせじばへびずろほげむべひょゆぶごゃねふぐぎぼゅづざぞぬぜぱぽぷぴぃぁぇぺゞぢぉぅゐゝゑ゛゜ゎゔ゚ゟ゙ゕゖ", false, false, ),
    LanguageEntry::new(Language::Portuguese, "aeosirdntmuclpgvbfhãqéçází", true, true, ),
    LanguageEntry::new(Language::Swedish, "eanrtsildomkgvhfupäcböåyjx", true, true, ),
    LanguageEntry::new(Language::Chinese, "的一是不了在人有我他这个们中来上大为和国地到以说时要就出会可也你对生能而子那得于着下自之年过发后作里用道行所然家种事成方多经么去法学如都同现当没动面起看定天分还进好小部其些主样理心她本前开但因只从想实", false, false, ),
    LanguageEntry::new(Language::Ukrainian, "оаніирвтесклудмпзяьбгйчхцї", false, false, ),
    LanguageEntry::new(Language::Norwegian, "erntasioldgkmvfpubhåyjøcæw", false, true, ),
    LanguageEntry::new(Language::Finnish, "aintesloukämrvjhpydögcbfwz", true, true, ),
    LanguageEntry::new(Language::Vietnamese, "nhticgaoumlràđsevpbyưdákộế", true, true, ),
    LanguageEntry::new(Language::Czech, "oeantsilvrkdumpíchzáyjběéř", true, true, ),
    LanguageEntry::new(Language::Hungarian, "eatlsnkriozáégmbyvdhupjöfc", true, true, ),
    LanguageEntry::new(Language::Korean, "이다에의는로하을가고지서한은기으년대사시를리도인스일", false, false, ),
    LanguageEntry::new(Language::Indonesian, "aneirtusdkmlgpbohyjcwfvzxq", false, true, ),
    LanguageEntry::new(Language::Turkish, "aeinrlıkdtsmyuobüşvgzhcpçğ", true, true, ),
    LanguageEntry::new(Language::Romanian, "eiarntulocsdpmăfvîgbșțzhâj", true, true, ),
    LanguageEntry::new(Language::Farsi, "ایردنهومتبسلکشزفگعخقجآپحطص", false, false, ),
    LanguageEntry::new(Language::Arabic, "اليمونرتبةعدسفهكقأحجشطصىخإ", false, false, ),
    LanguageEntry::new(Language::Danish, "erntaisdlogmkfvubhpåyøæcjw", false, true, ),
    LanguageEntry::new(Language::Serbian, "аиоенрсуткјвдмплгзбaieonцш", false, false, ),
    LanguageEntry::new(Language::Lithuanian, "iasoretnukmlpvdjgėbyųšžcąį", false, true, ),
    LanguageEntry::new(Language::Slovene, "eaionrsltjvkdpmuzbghčcšžfy", false, true, ),
    LanguageEntry::new(Language::Slovak, "oaenirvtslkdmpuchjbzáyýíčé", true, true, ),
    LanguageEntry::new(Language::Hebrew, "יוהלרבתמאשנעםדקחפסכגטצןזך", false, false, ),
    LanguageEntry::new(Language::Bulgarian, "аиоентрсвлкдпмзгяъубчцйжщх", false, false, ),
    LanguageEntry::new(Language::Croatian, "aioenrjstuklvdmpgzbcčhšžćf", true, true, ),
    LanguageEntry::new(Language::Hindi, "करसनतमहपयलवजदगबशटअएथभडचधषइ", false, false, ),
    LanguageEntry::new(Language::Estonian, "aiestlunokrdmvgpjhäbõüfcöy", true, true, ),
    LanguageEntry::new(Language::Thai, "านรอกเงมยลวดทสตะปบคหแจพชขใ", false, false, ),
    LanguageEntry::new(Language::Greek, "ατοιενρσκηπςυμλίόάγέδήωχθύ", false, false, ),
    LanguageEntry::new(Language::Tamil, "கதபடரமலனவறயளசநஇணஅஆழஙஎஉஒஸ", false, false, ),
    LanguageEntry::new(Language::Kazakh, "аыентрлідсмқкобиуғжңзшйпгө", false, false, ),
  ]
});

pub(crate) static ENCODING_TO_LANGUAGE: Lazy<HashMap<&'static str, Language>> = Lazy::new(|| {
    HashMap::from_iter([
        ("euc-kr", Language::Korean),
        ("big5", Language::Chinese),
        ("hz", Language::Chinese),
        ("gbk", Language::Chinese),
        ("gb18030", Language::Chinese),
        ("euc-jp", Language::Japanese),
        ("iso-2022-jp", Language::Japanese),
        ("shift_jis", Language::Japanese),
    ])
});

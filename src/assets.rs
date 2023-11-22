use crate::entity::Language;
use ahash::HashMap;

use once_cell::sync::Lazy;
use std::iter::FromIterator;

pub(crate) static LANGUAGES: Lazy<[(Language, &'static str, bool, bool); 41]> = Lazy::new(|| {
    [
  // language, alphabet, have_accents, pure_latin
  (Language::English, "eationsrhldcmufpgwbyvkjxzq", false, true, ),
  (Language::English, "eationsrhldcumfpgwybvkxjzq", false, true, ),
  (Language::German, "enirstadhulgocmbfkwzpvüäöj", true, true, ),
  (Language::French, "easnitrluodcpmévgfbhqàxèyj", true, true, ),
  (Language::Dutch, "enairtodslghvmukcpbwjzfyxë", true, true, ),
  (Language::Italian, "eiaonltrscdupmgvfbzhqèàkyò", true, true, ),
  (Language::Polish, "aioenrzwsctkydpmuljłgbhąęó", true, true, ),
  (Language::Spanish, "eaonsrildtcumpbgvfyóhqíjzá", true, true, ),
  (Language::Russian, "оаеинстрвлкмдпугяызбйьчхжц", false, false, ),
  (Language::Japanese, "人一大亅丁丨竹笑口日今二彳行十土丶寸寺時乙丿乂气気冂巾亠市目儿見八小凵県月彐門間木東山出本中刀分耳又取最言田心思刂前京尹事生厶云会未来白冫楽灬馬尸尺駅明耂者了阝都高卜占厂广店子申奄亻俺上方冖学衣艮食自", false, false, ),
  (Language::Japanese, "ーンス・ルトリイアラックドシレジタフロカテマィグバムプオコデニウメサビナブャエュチキズダパミェョハセベガモツネボソノァヴワポペピケゴギザホゲォヤヒユヨヘゼヌゥゾヶヂヲヅヵヱヰヮヽ゠ヾヷヿヸヹヺ", false, false, ),
  (Language::Japanese, "のにるたとはしいをでてがなれからさっりすあもこまうくよきんめおけそつだやえどわちみせじばへびずろほげむべひょゆぶごゃねふぐぎぼゅづざぞぬぜぱぽぷぴぃぁぇぺゞぢぉぅゐゝゑ゛゜ゎゔ゚ゟ゙ゕゖ", false, false, ),
  (Language::Portuguese, "aeosirdntmuclpgvbfhãqéçází", true, true, ),
  (Language::Swedish, "eanrtsildomkgvhfupäcböåyjx", true, true, ),
  (Language::Chinese, "的一是不了在人有我他这个们中来上大为和国地到以说时要就出会可也你对生能而子那得于着下自之年过发后作里用道行所然家种事成方多经么去法学如都同现当没动面起看定天分还进好小部其些主样理心她本前开但因只从想实", false, false, ),
  (Language::Ukrainian, "оаніирвтесклудмпзяьбгйчхцї", false, false, ),
  (Language::Norwegian, "erntasioldgkmvfpubhåyjøcæw", false, true, ),
  (Language::Finnish, "aintesloukämrvjhpydögcbfwz", true, true, ),
  (Language::Vietnamese, "nhticgaoumlràđsevpbyưdákộế", true, true, ),
  (Language::Czech, "oeantsilvrkdumpíchzáyjběéř", true, true, ),
  (Language::Hungarian, "eatlsnkriozáégmbyvdhupjöfc", true, true, ),
  (Language::Korean, "이다에의는로하을가고지서한은기으년대사시를리도인스일", false, false, ),
  (Language::Indonesian, "aneirtusdkmlgpbohyjcwfvzxq", false, true, ),
  (Language::Turkish, "aeinrlıkdtsmyuobüşvgzhcpçğ", true, true, ),
  (Language::Romanian, "eiarntulocsdpmăfvîgbșțzhâj", true, true, ),
  (Language::Farsi, "ایردنهومتبسلکشزفگعخقجآپحطص", false, false, ),
  (Language::Arabic, "اليمونرتبةعدسفهكقأحجشطصىخإ", false, false, ),
  (Language::Danish, "erntaisdlogmkfvubhpåyøæcjw", false, true, ),
  (Language::Serbian, "аиоенрсуткјвдмплгзбaieonцш", false, false, ),
  (Language::Lithuanian, "iasoretnukmlpvdjgėbyųšžcąį", false, true, ),
  (Language::Slovene, "eaionrsltjvkdpmuzbghčcšžfy", false, true, ),
  (Language::Slovak, "oaenirvtslkdmpuchjbzáyýíčé", true, true, ),
  (Language::Hebrew, "יוהלרבתמאשנעםדקחפסכגטצןזך", false, false, ),
  (Language::Bulgarian, "аиоентрсвлкдпмзгяъубчцйжщх", false, false, ),
  (Language::Croatian, "aioenrjstuklvdmpgzbcčhšžćf", true, true, ),
  (Language::Hindi, "करसनतमहपयलवजदगबशटअएथभडचधषइ", false, false, ),
  (Language::Estonian, "aiestlunokrdmvgpjhäbõüfcöy", true, true, ),
  (Language::Thai, "านรอกเงมยลวดทสตะปบคหแจพชขใ", false, false, ),
  (Language::Greek, "ατοιενρσκηπςυμλίόάγέδήωχθύ", false, false, ),
  (Language::Tamil, "கதபடரமலனவறயளசநஇணஅஆழஙஎஉஒஸ", false, false, ),
  (Language::Kazakh, "аыентрлідсмқкобиуғжңзшйпгө", false, false, ),
]
});
pub(crate) static LANGUAGE_SUPPORTED_COUNT: Lazy<usize> = Lazy::new(|| LANGUAGES.len()); // 41

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

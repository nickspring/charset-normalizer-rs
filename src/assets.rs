use lazy_static::lazy_static;
use maplit::hashmap;
use std::collections::HashMap;

lazy_static! {
   pub static ref FREQUENCIES: HashMap<&'static str, &'static str> = hashmap!{
      "English" => "eationsrhldcumfpgwybvkxjzq",
      "English—" => "eationsrhldcmufpgwbyvkjxzq",
      "German" => "enirstadhulgocmbfkwzpvüäöj",
      "French" => "easnitrluodcpmévgfbhqàxèyj",
      "Dutch" => "enairtodslghvmukcpbwjzfyxë",
      "Italian" => "eiaonltrscdupmgvfbzhqèàkyò",
      "Polish" => "aioenrzwsctkydpmuljłgbhąęó",
      "Spanish" => "eaonsrildtcumpbgvfyóhqíjzá",
      "Russian" => "оаеинстрвлкмдпугяызбйьчхжц",
      // Jap-Kanji
      "Japanese" => "人一大亅丁丨竹笑口日今二彳行十土丶寸寺時乙丿乂气気冂巾亠市目儿見八小凵県月彐門間木東山出本中刀分耳又取最言田心思刂前京尹事生厶云会未来白冫楽灬馬尸尺駅明耂者了阝都高卜占厂广店子申奄亻俺上方冖学衣艮食自",
      // Jap-Katakana
      "Japanese—" => "ーンス・ルトリイアラックドシレジタフロカテマィグバムプオコデニウメサビナブャエュチキズダパミェョハセベガモツネボソノァヴワポペピケゴギザホゲォヤヒユヨヘゼヌゥゾヶヂヲヅヵヱヰヮヽ゠ヾヷヿヸヹヺ",
      // Jap-Hiragana
      "Japanese——" => "のにるたとはしいをでてがなれからさっりすあもこまうくよきんめおけそつだやえどわちみせじばへびずろほげむべひょゆぶごゃねふぐぎぼゅづざぞぬぜぱぽぷぴぃぁぇぺゞぢぉぅゐゝゑ゛゜ゎゔ゚ゟ゙ゕゖ",
      "Portuguese" => "aeosirdntmuclpgvbfhãqéçází",
      "Swedish" => "eanrtsildomkgvhfupäcböåyjx",
      "Chinese" => "的一是不了在人有我他这个们中来上大为和国地到以说时要就出会可也你对生能而子那得于着下自之年过发后作里用道行所然家种事成方多经么去法学如都同现当没动面起看定天分还进好小部其些主样理心她本前开但因只从想实",
      "Ukrainian" => "оаніирвтесклудмпзяьбгйчхцї",
      "Norwegian" => "erntasioldgkmvfpubhåyjøcæw",
      "Finnish" => "aintesloukämrvjhpydögcbfwz",
      "Vietnamese" => "nhticgaoumlràđsevpbyưdákộế",
      "Czech" => "oeantsilvrkdumpíchzáyjběéř",
      "Hungarian" => "eatlsnkriozáégmbyvdhupjöfc",
      "Korean" => "이다에의는로하을가고지서한은기으년대사시를리도인스일",
      "Indonesian" => "aneirtusdkmlgpbohyjcwfvzxq",
      "Turkish" => "aeinrlıkdtsmyuobüşvgzhcpçğ",
      "Romanian" => "eiarntulocsdpmăfvîgbșțzhâj",
      "Farsi" => "ایردنهومتبسلکشزفگعخقجآپحطص",
      "Arabic" => "اليمونرتبةعدسفهكقأحجشطصىخإ",
      "Danish" => "erntaisdlogmkfvubhpåyøæcjw",
      "Serbian" => "аиоенрсуткјвдмплгзбaieonцш",
      "Lithuanian" => "iasoretnukmlpvdjgėbyųšžcąį",
      "Slovene" => "eaionrsltjvkdpmuzbghčcšžfy",
      "Slovak" => "oaenirvtslkdmpuchjbzáyýíčé",
      "Hebrew" => "יוהלרבתמאשנעםדקחפסכגטצןזך",
      "Bulgarian" => "аиоентрсвлкдпмзгяъубчцйжщх",
      "Croatian" => "aioenrjstuklvdmpgzbcčhšžćf",
      "Hindi" => "करसनतमहपयलवजदगबशटअएथभडचधषइ",
      "Estonian" => "aiestlunokrdmvgpjhäbõüfcöy",
      "Thai" => "านรอกเงมยลวดทสตะปบคหแจพชขใ",
      "Greek" => "ατοιενρσκηπςυμλίόάγέδήωχθύ",
      "Tamil" => "கதபடரமலனவறயளசநஇணஅஆழஙஎஉஒஸ",
      "Kazakh" => "аыентрлідсмқкобиуғжңзшйпгө",
   };

   pub static ref LANGUAGE_SUPPORTED_COUNT: usize = FREQUENCIES.len();

}

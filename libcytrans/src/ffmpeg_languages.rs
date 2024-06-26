// all data in this file was obtained from Wikipedia
// https://en.wikipedia.org/wiki/IETF_language_tag
// https://en.wikipedia.org/wiki/List_of_ISO_639-1_codes

use once_cell::sync::Lazy;
use std::collections::HashMap;

// table mapping languages to their english names
pub static ENGLISH_NAMES: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| HashMap::from([
    ("abk", "Abkhazian"),
    ("aar", "Afar"),
    ("afr", "Afrikaans"),
    ("aka", "Akan"),
    ("alb", "Albanian"),
    ("amh", "Amharic"),
    ("ara", "Arabic"),
    ("arg", "Aragonese"),
    ("arm", "Armenian"),
    ("asm", "Assamese"),
    ("ava", "Avaric"),
    ("ave", "Avestan"),
    ("aym", "Aymara"),
    ("aze", "Azerbaijani"),
    ("bam", "Bambara"),
    ("bak", "Bashkir"),
    ("baq", "Basque"),
    ("bel", "Belarusian"),
    ("ben", "Bengali"),
    ("bis", "Bislama"),
    ("bos", "Bosnian"),
    ("bre", "Breton"),
    ("bul", "Bulgarian"),
    ("bur", "Burmese"),
    ("cat", "Catalan"),
    ("cha", "Chamorro"),
    ("che", "Chechen"),
    ("nya", "Nyanja"),
    ("chi", "Chinese"),
    ("chu", "Church Slavonic"),
    ("chv", "Chuvash"),
    ("cor", "Cornish"),
    ("cos", "Corsican"),
    ("cre", "Cree"),
    ("hrv", "Croatian"),
    ("cze", "Czech"),
    ("dan", "Danish"),
    ("div", "Divehi"),
    ("dut", "Dutch"),
    ("dzo", "Dzongkha"),
    ("eng", "English"),
    ("epo", "Esperanto"),
    ("est", "Estonian"),
    ("ewe", "Ewe"),
    ("fao", "Faroese"),
    ("fij", "Fijian"),
    ("fin", "Finnish"),
    ("fre", "French"),
    ("fry", "Western Frisian"),
    ("ful", "Fulah"),
    ("gla", "Gaelic"),
    ("glg", "Galician"),
    ("lug", "Ganda"),
    ("geo", "Georgian"),
    ("ger", "German"),
    ("gre", "Greek"),
    ("kal", "Kalaallisut"),
    ("grn", "Guarani"),
    ("guj", "Gujarati"),
    ("hat", "Haitian"),
    ("hau", "Hausa"),
    ("heb", "Hebrew"),
    ("her", "Herero"),
    ("hin", "Hindi"),
    ("hmo", "Hiri Motu"),
    ("hun", "Hungarian"),
    ("ice", "Icelandic"),
    ("ido", "Ido"),
    ("ibo", "Igbo"),
    ("ind", "Indonesian"),
    ("ina", "Interlingua"),
    ("ile", "Interlingue"),
    ("iku", "Inuktitut"),
    ("ipk", "Inupiaq"),
    ("gle", "Irish"),
    ("ita", "Italian"),
    ("jpn", "Japanese"),
    ("jav", "Javanese"),
    ("kan", "Kannada"),
    ("kau", "Kanuri"),
    ("kas", "Kashmiri"),
    ("kaz", "Kazakh"),
    ("khm", "Central Khmer"),
    ("kik", "Kikuyu, Gikuyu"),
    ("kin", "Kinyarwanda"),
    ("kir", "Kirghiz, Kyrgyz"),
    ("kom", "Komi"),
    ("kon", "Kongo"),
    ("kor", "Korean"),
    ("kua", "Kuanyama, Kwanyama"),
    ("kur", "Kurdish"),
    ("lao", "Lao"),
    ("lat", "Latin"),
    ("lav", "Latvian"),
    ("lim", "Limburgan"),
    ("lin", "Lingala"),
    ("lit", "Lithuanian"),
    ("lub", "Luba-Katanga"),
    ("ltz", "Luxembourgish, Letzeburgesch"),
    ("mac", "Macedonian"),
    ("mlg", "Malagasy"),
    ("may", "Malay"),
    ("mal", "Malayalam"),
    ("mlt", "Maltese"),
    ("glv", "Manx"),
    ("mao", "Maori"),
    ("mar", "Marathi"),
    ("mah", "Marshallese"),
    ("mon", "Mongolian"),
    ("nau", "Nauru"),
    ("nav", "Navajo, Navaho"),
    ("nde", "North Ndebele"),
    ("nbl", "South Ndebele"),
    ("ndo", "Ndonga"),
    ("nep", "Nepali"),
    ("nor", "Norwegian"),
    ("nob", "Norwegian Bokmål"),
    ("nno", "Norwegian Nynorsk"),
    ("iii", "Sichuan Yi, Nuosu"),
    ("oci", "Occitan"),
    ("oji", "Ojibwa"),
    ("ori", "Oriya"),
    ("orm", "Oromo"),
    ("oss", "Ossetian"),
    ("pli", "Pali"),
    ("pus", "Pashto"),
    ("per", "Persian"),
    ("pol", "Polish"),
    ("por", "Portuguese"),
    ("pan", "Punjabi"),
    ("que", "Quechua"),
    ("rum", "Romanian"),
    ("roh", "Romansh"),
    ("run", "Rundi"),
    ("rus", "Russian"),
    ("sme", "Northern Sami"),
    ("smo", "Samoan"),
    ("sag", "Sango"),
    ("san", "Sanskrit"),
    ("srd", "Sardinian"),
    ("srp", "Serbian"),
    ("sna", "Shona"),
    ("snd", "Sindhi"),
    ("sin", "Sinhala, Sinhalese"),
    ("slo", "Slovak"),
    ("slv", "Slovenian"),
    ("som", "Somali"),
    ("sot", "Southern Sotho"),
    ("spa", "Spanish"),
    ("sun", "Sundanese"),
    ("swa", "Swahili"),
    ("ssw", "Swati"),
    ("swe", "Swedish"),
    ("tgl", "Tagalog"),
    ("tah", "Tahitian"),
    ("tgk", "Tajik"),
    ("tam", "Tamil"),
    ("tat", "Tatar"),
    ("tel", "Telugu"),
    ("tha", "Thai"),
    ("tib", "Tibetan"),
    ("tir", "Tigrinya"),
    ("ton", "Tonga (Tonga Islands)"),
    ("tso", "Tsonga"),
    ("tsn", "Tswana"),
    ("tur", "Turkish"),
    ("tuk", "Turkmen"),
    ("twi", "Twi"),
    ("uig", "Uyghur"),
    ("ukr", "Ukrainian"),
    ("urd", "Urdu"),
    ("uzb", "Uzbek"),
    ("ven", "Venda"),
    ("vie", "Vietnamese"),
    ("vol", "Volapük"),
    ("wln", "Walloon"),
    ("wel", "Welsh"),
    ("wol", "Wolof"),
    ("xho", "Xhosa"),
    ("yid", "Yiddish"),
    ("yor", "Yoruba"),
    ("zha", "Zhuang"),
    ("zul", "Zulu"),
]));

// table mapping languages to their localized names

pub static LANGUAGES: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| HashMap::from([
    ("afr", "Afrikaans"), // Afrikaans
    ("amh", "አማርኛ"), // Amharic
    ("ara", "العربية"), // Arabic
    ("aze", "Azərbaycan­lı"), // Azerbaijani
    ("bak", "Башҡорт"), // Bashkir
    ("bel", "беларуская"), // Belarusian
    ("bul", "български"), // Bulgarian
    ("ben", "বাং "), // Bengali
    ("tib", "བོད་ཡིག"), // Tibetan
    ("bre", "brezhoneg"), // Breton
    ("bos", "bosanski/босански"), // Bosnian
    ("cos", "Corsu"), // Corsican
    ("cze", "čeština"), // Czech
    ("wel", "Cymraeg"), // Welsh
    ("dan", "dansk"), // Danish
    ("ger", "Deutsch"), // German
    ("eng", "English"), // English
    ("est", "eesti"), // Estonian
    ("baq", "euskara"), // Basque
    ("per", "فارسى"), // Persian
    ("fin", "suomi"), // Finnish
    ("fao", "føroyskt"), // Faroese
    ("fre", "français"), // French
    ("gle", "Gaeilge"), // Irish
    ("glg", "galego"), // Galician
    ("guj", "ગુજરાતી"), // Gujarati
    ("hau", "Hausa"), // Hausa
    ("heb", "עברית"), // Hebrew
    ("hin", "हिंदी"), // Hindi
    ("hrv", "hrvatski"), // Croatian
    ("hun", "magyar"), // Hungarian
    ("arm", "Հայերեն"), // Armenian
    ("ind", "Bahasa Indonesia"), // Indonesian
    ("ibo", "Igbo"), // Igbo
    ("ice", "íslenska"), // Icelandic
    ("ita", "italiano"), // Italian
    ("iku", "Inuktitut /ᐃᓄᒃᑎᑐᑦ (ᑲᓇᑕ)"), // Inuktitut
    ("jpn", "日本語"), // Japanese
    ("geo", "ქართული"), // Georgian
    ("kaz", "Қазақша"), // Kazakh
    ("kan", "ಕನ್ನಡ"), // Kannada
    ("kor", "한국어"), // Korean
    ("lao", "ລາວ"), // Lao
    ("lit", "lietuvių"), // Lithuanian
    ("lav", "latviešu"), // Latvian
    ("mao", "Reo Māori"), // Maori
    ("mac", "македонски јазик"), // Macedonian
    ("mal", "മലയാളം"), // Malayalam
    ("mon", "Монгол хэл/ᠮᠤᠨᠭᠭᠤᠯ ᠬᠡᠯᠡ"), // Mongolian
    ("mar", "मराठी"), // Marathi
    ("may", "Bahasa Malaysia"), // Malay
    ("mlt", "Malti"), // Maltese
    ("bur", "Myanmar"), // Burmese
    ("nep", "नेपाली (नेपाल)"), // Nepali
    ("nor", "norsk"), // Norwegian
    ("oci", "Occitan"), // Occitan
    ("pol", "polski"), // Polish
    ("por", "Português"), // Portuguese
    ("que", "runasimi"), // Quechua
    ("roh", "Rumantsch"), // Romansh
    ("rus", "русский"), // Russian
    ("kin", "Kinyarwanda"), // Kinyarwanda
    ("san", "संस्कृत"), // Sanskrit
    ("slo", "slovenčina"), // Slovak
    ("slv", "slovenski"), // Slovenian
    ("alb", "shqipe"), // Albanian
    ("srp", "srpski/српски"), // Serbian
    ("swe", "svenska"), // Swedish
    ("tam", "தமிழ்"), // Tamil
    ("tel", "తెలుగు"), // Telugu
    ("tgk", "Тоҷикӣ"), // Tajik
    ("tha", "ไทย"), // Thai
    ("tuk", "türkmençe"), // Turkmen
    ("tsn", "Setswana"), // Tswana
    ("tur", "Türkçe"), // Turkish
    ("tat", "Татарча"), // Tatar
    ("ukr", "українська"), // Ukrainian
    ("urd", "اُردو"), // Urdu
    ("uzb", "U'zbek/Ўзбек"), // Uzbek
    ("vie", "Tiếng Việt/㗂越"), // Vietnamese
    ("wol", "Wolof"), // Wolof
    ("xho", "isiXhosa"), // Xhosa
    ("yor", "Yoruba"), // Yoruba
    ("chi", "中文"), // Chinese
    ("zul", "isiZulu"), // Zulu
]));

// Maps ISO 639-2/B language codes used by ffmpeg to IETF language tags used by Cytube.
pub(crate) static FF2CT: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| HashMap::from([
    ("afr", "af"), // Afrikaans
    ("amh", "am"), // Amharic
    ("ara", "ar"), // Arabic
    ("asm", "as"), // Assamese
    ("aze", "az"), // Azerbaijani
    ("bak", "ba"), // Bashkir
    ("bel", "be"), // Belarusian
    ("bul", "bg"), // Bulgarian
    ("ben", "bn"), // Bengali
    ("tib", "bo"), // Tibetan
    ("bre", "br"), // Breton
    ("bos", "bs"), // Bosnian
    ("cos", "co"), // Corsican
    ("cze", "cs"), // Czech
    ("wel", "cy"), // Welsh
    ("dan", "da"), // Danish
    ("ger", "de"), // German
    ("eng", "en"), // English
    ("est", "et"), // Estonian
    ("baq", "eu"), // Basque
    ("per", "fa"), // Persian
    ("fin", "fi"), // Finnish
    ("fao", "fo"), // Faroese
    ("fre", "fr"), // French
    ("gle", "ga"), // Irish
    ("glg", "gl"), // Galician
    ("guj", "gu"), // Gujarati
    ("hau", "ha"), // Hausa
    ("heb", "he"), // Hebrew
    ("hin", "hi"), // Hindi
    ("hrv", "hr"), // Croatian
    ("hun", "hu"), // Hungarian
    ("arm", "hy"), // Armenian
    ("ind", "id"), // Indonesian
    ("ibo", "ig"), // Igbo
    ("ice", "is"), // Icelandic
    ("ita", "it"), // Italian
    ("iku", "iu"), // Inuktitut
    ("jpn", "ja"), // Japanese
    ("geo", "ka"), // Georgian
    ("kaz", "kk"), // Kazakh
    ("kan", "kn"), // Kannada
    ("kor", "ko"), // Korean
    ("lao", "lo"), // Lao
    ("lit", "lt"), // Lithuanian
    ("lav", "lv"), // Latvian
    ("mao", "mi"), // Maori
    ("mac", "mk"), // Macedonian
    ("mal", "ml"), // Malayalam
    ("mon", "mn"), // Mongolian
    ("mar", "mr"), // Marathi
    ("may", "ms"), // Malay
    ("mlt", "mt"), // Maltese
    ("bur", "my"), // Burmese
    ("nep", "ne"), // Nepali
    ("nor", "no"), // Norwegian
    ("oci", "oc"), // Occitan
    ("pol", "pl"), // Polish
    ("por", "pt"), // Portuguese
    ("que", "qu"), // Quechua
    ("roh", "rm"), // Romansh
    ("rus", "ru"), // Russian
    ("kin", "rw"), // Kinyarwanda
    ("san", "sa"), // Sanskrit
    ("slo", "sk"), // Slovak
    ("slv", "sl"), // Slovenian
    ("alb", "sq"), // Albanian
    ("srp", "sr"), // Serbian
    ("swe", "sv"), // Swedish
    ("tam", "ta"), // Tamil
    ("tel", "te"), // Telugu
    ("tgk", "tg"), // Tajik
    ("tha", "th"), // Thai
    ("tuk", "tk"), // Turkmen
    ("tsn", "tn"), // Tswana
    ("tur", "tr"), // Turkish
    ("tat", "tt"), // Tatar
    ("ukr", "uk"), // Ukrainian
    ("urd", "ur"), // Urdu
    ("uzb", "uz"), // Uzbek
    ("vie", "vi"), // Vietnamese
    ("wol", "wo"), // Wolof
    ("xho", "xh"), // Xhosa
    ("yor", "yo"), // Yoruba
    ("chi", "zh"), // Chinese
    ("zul", "zu"), // Zulu
]));

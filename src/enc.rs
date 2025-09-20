use encoding::CodecError;
use encoding::DecoderTrap;
use encoding::EncodingRef;
use encoding::StringWriter;
use encoding_rs::DecoderResult;
use encoding_rs::Encoding as EncodingImpl;
use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::collections::HashMap;

#[derive(Copy, Clone)]
pub(crate) struct Encoding {
    /// Canonical name
    name: &'static str,
    is_multi_byte_encoding: bool,
    /// Acceptable aliases from <https://encoding.spec.whatwg.org/#concept-encoding-get> -> as is + lowercased
    aliases: &'static [&'static str],

    legacy_encoder: EncodingRef,
    encoder_impl: Option<&'static EncodingImpl>,
}

impl std::fmt::Display for Encoding {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.name.fmt(fmt)
    }
}

impl std::fmt::Debug for Encoding {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.name.fmt(fmt)
    }
}

impl PartialEq for Encoding {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Encoding {}

impl std::hash::Hash for Encoding {
    fn hash<H>(&self, h: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.name.hash(h)
    }
}

// Test Decoding bytes to string with specified encoding without writing result to memory
// returns true if everything is correctly decoded, otherwise false
struct DecodeTestResult {
    only_test: bool,
    data: String,
}
impl StringWriter for DecodeTestResult {
    fn writer_hint(&mut self, expectedlen: usize) {
        if self.only_test {
            return;
        }
        let newlen = self.data.len() + expectedlen;
        self.data.reserve(newlen);
    }
    fn write_char(&mut self, c: char) {
        if self.only_test {
            return;
        }
        self.data.push(c);
    }
    fn write_str(&mut self, s: &str) {
        if self.only_test {
            return;
        }
        self.data.push_str(s);
    }
}
impl DecodeTestResult {
    pub fn get_buffer(&self) -> &str {
        &self.data
    }
}

// Copied implementation of decode_to from encoder lib
// (we need index of problematic chars & hacks for chunks)
fn decode_to(
    encoder: EncodingRef,
    input: &[u8],
    ret: &mut dyn StringWriter,
) -> Result<(), CodecError> {
    let mut decoder = encoder.raw_decoder();
    let mut remaining = 0;
    let trap = DecoderTrap::Strict;
    loop {
        let (offset, err) = decoder.raw_feed(&input[remaining..], ret);
        let unprocessed = remaining + offset;

        match err {
            Some(err) => {
                remaining = remaining.wrapping_add_signed(err.upto);
                if !trap.trap(&mut *decoder, &input[unprocessed..remaining], ret) {
                    return Err(err);
                }
            }
            None => {
                remaining = input.len();
                if let Some(err) = decoder.raw_finish(ret) {
                    remaining = remaining.wrapping_add_signed(err.upto);
                    if !trap.trap(&mut *decoder, &input[unprocessed..remaining], ret) {
                        return Err(err);
                    }
                }
                if remaining >= input.len() {
                    return Ok(());
                }
            }
        }
    }
}

/// Whether the input should be processed chunk-wise.
/// If so, the decode will nibble off the start/end
/// of the buffer to find a subset that successfully
/// decodes
#[derive(PartialEq, Copy, Clone, Debug)]
pub(crate) enum IsChunk {
    Yes,
    No,
}

/// Whether the full decoded output is required.
/// If not, memory utilization can be reduced by
/// using smaller or no buffer to hold the decoded
/// result; an empty or partial string will be
/// returned from the decode
#[derive(PartialEq, Copy, Clone, Debug)]
pub(crate) enum WantDecode {
    Yes,
    No,
}

impl Encoding {
    pub fn by_name(name: &str) -> Option<&'static Encoding> {
        match BY_NAME.get(name) {
            Some(enc) => Some(enc),
            None => {
                if name.chars().any(|c| c.is_ascii_uppercase()) {
                    Self::by_name(&name.to_lowercase())
                } else {
                    None
                }
            }
        }
    }

    pub fn aliases(&self) -> &'static [&'static str] {
        self.aliases
    }

    pub fn name(&self) -> &str {
        self.name
    }

    pub fn requires_bom(&self) -> bool {
        matches!(self.name, "utf-16le" | "utf-16be")
    }

    pub fn is_multi_byte_encoding(&self) -> bool {
        self.is_multi_byte_encoding
    }

    pub fn decode(
        &self,
        input: &[u8],
        want_decode: WantDecode,
        is_chunk: IsChunk,
    ) -> Result<String, String> {
        let result = self.decode_updated(input, want_decode, is_chunk);
        if false {
            let legacy = self.decode_legacy(
                input,
                want_decode == WantDecode::No,
                is_chunk == IsChunk::Yes,
            );
            assert_eq!(
                legacy.is_ok(),
                result.is_ok(),
                "mismatch: {self} want_decode={want_decode:?} \
                is_chunk={is_chunk:?} legacy={legacy:#?}\nupdated={result:#?}"
            );
        }
        result
    }

    fn decode_updated(
        &self,
        input: &[u8],
        want_decode: WantDecode,
        is_chunk: IsChunk,
    ) -> Result<String, String> {
        match self.encoder_impl {
            // The ascii special case
            None => {
                let len = input.len();
                let valid_to = encoding_rs::Encoding::ascii_valid_up_to(input);
                if valid_to != len {
                    Err(format!("8-bit input detected as index {valid_to}"))
                } else {
                    match want_decode {
                        WantDecode::Yes => Ok(std::str::from_utf8(input)
                            .map_err(|err| format!("{err:#}"))?
                            .to_string()),
                        WantDecode::No => Ok(String::new()),
                    }
                }
            }
            Some(enc) => {
                let mut begin_offset = 0;
                let mut end_offset = input.len();

                loop {
                    let chunk = &input[begin_offset..end_offset];

                    let mut decoder = enc.new_decoder();

                    // TODO: it should be technically possible to cap the buffer
                    // size when WantDecode::No, but it means using a slightly
                    // more complex decoder method and state tracking than we
                    // are currently
                    let mut result_string = String::with_capacity(
                        decoder
                            .max_utf8_buffer_length_without_replacement(input.len())
                            .unwrap_or(0),
                    );

                    let is_last = true;
                    let (result, consumed) = decoder.decode_to_string_without_replacement(
                        chunk,
                        &mut result_string,
                        is_last,
                    );

                    match result {
                        DecoderResult::InputEmpty if is_last => {
                            return match want_decode {
                                WantDecode::Yes => Ok(result_string),
                                WantDecode::No => Ok(String::new()),
                            }
                        }
                        DecoderResult::InputEmpty => return Err("more input needed".to_string()),
                        DecoderResult::OutputFull => {
                            return Err("result buffer not big enough".to_string())
                        }
                        DecoderResult::Malformed(len, consumed_after) => {
                            let mut terminate = false;
                            match is_chunk {
                                IsChunk::Yes => {
                                    if consumed <= 1 {
                                        // Bad sequence at the start
                                        begin_offset += (len + consumed_after).max(1) as usize;
                                    } else {
                                        end_offset = end_offset.saturating_sub(1);
                                    }

                                    if end_offset - begin_offset < 1
                                        || begin_offset > 3
                                        || input.len() - end_offset > 3
                                    {
                                        terminate = true;
                                    }
                                }
                                IsChunk::No => {
                                    terminate = true;
                                }
                            }

                            if terminate {
                                if consumed <= 1 {
                                    return Err(format!("invalid sequence at {consumed}"));
                                }
                                return Err(format!("incomplete sequence at {consumed}"));
                            }
                        }
                    }
                }
            }
        }
    }

    pub(crate) fn decode_legacy(
        &self,
        input: &[u8],
        only_test: bool,
        is_chunk: bool,
    ) -> Result<String, String> {
        let mut buf = DecodeTestResult {
            only_test,
            data: String::new(),
        };
        let mut err = CodecError {
            upto: 0,
            cause: Cow::from(String::new()),
        };
        let chunk_len = input.len();
        let mut begin_offset: usize = 0;
        let mut end_offset: usize = chunk_len;
        let mut error_occured: bool;
        loop {
            let res = decode_to(
                self.legacy_encoder,
                &input[begin_offset..end_offset],
                &mut buf,
            );
            error_occured = res.is_err();
            if !is_chunk || res.is_ok() || !self.is_multi_byte_encoding {
                break;
            }
            err = res.unwrap_err();
            if err.cause.contains("invalid sequence") {
                begin_offset += 1;
            } else if err.cause.contains("incomplete sequence") {
                end_offset -= 1;
            }
            if end_offset - begin_offset < 1 || begin_offset > 3 || (chunk_len - end_offset) > 3 {
                break;
            }
        }
        if error_occured {
            return Err(format!("{} at index {}", err.cause, err.upto));
        }
        Ok(String::from(buf.get_buffer()))
    }
}

pub(crate) static BY_NAME: Lazy<HashMap<&'static str, &'static Encoding>> = Lazy::new(|| {
    let mut map = HashMap::new();
    for enc in ALL {
        for &name in enc.aliases {
            map.insert(name, enc);
        }
    }
    map
});

pub(crate) static ALL: &[Encoding] = &[
    Encoding {
        // See comments in windows-1252 below re: ascii aliasing with cp1252
        // and why that isn't the case here
        name: "ascii",
        is_multi_byte_encoding: false,
        aliases: &["ascii", "us-ascii"],
        legacy_encoder: encoding::all::ASCII,
        encoder_impl: None,
    },
    Encoding {
        name: "ibm866",
        is_multi_byte_encoding: false,
        aliases: &["866", "cp866", "csibm866", "ibm866"],
        legacy_encoder: encoding::all::IBM866,
        encoder_impl: Some(encoding_rs::IBM866),
    },
    Encoding {
        name: "iso-8859-2",
        is_multi_byte_encoding: false,
        aliases: &[
            "csisolatin2",
            "iso-8859-2",
            "iso-ir-101",
            "iso8859-2",
            "iso88592",
            "iso_8859-2",
            "iso_8859-2:1987",
            "l2",
            "latin2",
        ],
        legacy_encoder: encoding::all::ISO_8859_2,
        encoder_impl: Some(encoding_rs::ISO_8859_2),
    },
    Encoding {
        name: "iso-8859-3",
        is_multi_byte_encoding: false,
        aliases: &[
            "csisolatin3",
            "iso-8859-3",
            "iso-ir-109",
            "iso8859-3",
            "iso88593",
            "iso_8859-3",
            "iso_8859-3:1988",
            "l3",
            "latin3",
        ],
        legacy_encoder: encoding::all::ISO_8859_3,
        encoder_impl: Some(encoding_rs::ISO_8859_3),
    },
    Encoding {
        name: "iso-8859-4",
        is_multi_byte_encoding: false,
        aliases: &[
            "csisolatin4",
            "iso-8859-4",
            "iso-ir-110",
            "iso8859-4",
            "iso88594",
            "iso_8859-4",
            "iso_8859-4:1988",
            "l4",
            "latin4",
        ],
        legacy_encoder: encoding::all::ISO_8859_4,
        encoder_impl: Some(encoding_rs::ISO_8859_4),
    },
    Encoding {
        name: "iso-8859-5",
        is_multi_byte_encoding: false,
        aliases: &[
            "csisolatincyrillic",
            "cyrillic",
            "iso-8859-5",
            "iso-ir-144",
            "iso8859-5",
            "iso88595",
            "iso_8859-5",
            "iso_8859-5:1988",
        ],
        legacy_encoder: encoding::all::ISO_8859_5,
        encoder_impl: Some(encoding_rs::ISO_8859_5),
    },
    Encoding {
        name: "iso-8859-6",
        is_multi_byte_encoding: false,
        aliases: &[
            "arabic",
            "asmo-708",
            "csiso88596e",
            "csiso88596i",
            "csisolatinarabic",
            "ecma-114",
            "iso-8859-6",
            "iso-8859-6-e",
            "iso-8859-6-i",
            "iso-ir-127",
            "iso8859-6",
            "iso88596",
            "iso_8859-6",
            "iso_8859-6:1987",
        ],
        legacy_encoder: encoding::all::ISO_8859_6,
        encoder_impl: Some(encoding_rs::ISO_8859_6),
    },
    Encoding {
        name: "iso-8859-7",
        is_multi_byte_encoding: false,
        aliases: &[
            "csisolatingreek",
            "ecma-118",
            "elot_928",
            "greek",
            "greek8",
            "iso-8859-7",
            "iso-ir-126",
            "iso8859-7",
            "iso88597",
            "iso_8859-7",
            "iso_8859-7:1987",
            "sun_eu_greek",
        ],
        legacy_encoder: encoding::all::ISO_8859_7,
        encoder_impl: Some(encoding_rs::ISO_8859_7),
    },
    Encoding {
        name: "iso-8859-8",
        is_multi_byte_encoding: false,
        aliases: &[
            "csiso88598e",
            "csisolatinhebrew",
            "hebrew",
            "iso-8859-8",
            "iso-8859-8-e",
            "iso-ir-138",
            "iso8859-8",
            "iso88598",
            "iso_8859-8",
            "iso_8859-8:1988",
            "visual",
        ],
        legacy_encoder: encoding::all::ISO_8859_8,
        encoder_impl: Some(encoding_rs::ISO_8859_8),
    },
    Encoding {
        name: "iso-8859-10",
        is_multi_byte_encoding: false,
        aliases: &[
            "csisolatin6",
            "iso-8859-10",
            "iso-ir-157",
            "iso8859-10",
            "iso885910",
            "l6",
            "latin6",
        ],
        legacy_encoder: encoding::all::ISO_8859_10,
        encoder_impl: Some(encoding_rs::ISO_8859_10),
    },
    Encoding {
        name: "iso-8859-13",
        is_multi_byte_encoding: false,
        aliases: &["iso-8859-13", "iso8859-13", "iso885913"],
        legacy_encoder: encoding::all::ISO_8859_13,
        encoder_impl: Some(encoding_rs::ISO_8859_13),
    },
    Encoding {
        name: "iso-8859-14",
        is_multi_byte_encoding: false,
        aliases: &["iso-8859-14", "iso8859-14", "iso885914"],
        legacy_encoder: encoding::all::ISO_8859_14,
        encoder_impl: Some(encoding_rs::ISO_8859_14),
    },
    Encoding {
        name: "iso-8859-15",
        is_multi_byte_encoding: false,
        aliases: &[
            "csisolatin9",
            "iso-8859-15",
            "iso8859-15",
            "iso885915",
            "iso_8859-15",
            "l9",
        ],
        legacy_encoder: encoding::all::ISO_8859_15,
        encoder_impl: Some(encoding_rs::ISO_8859_15),
    },
    Encoding {
        name: "iso-8859-16",
        is_multi_byte_encoding: false,
        aliases: &["iso-8859-16"],
        legacy_encoder: encoding::all::ISO_8859_16,
        encoder_impl: Some(encoding_rs::ISO_8859_16),
    },
    Encoding {
        name: "koi8-r",
        is_multi_byte_encoding: false,
        aliases: &["cskoi8r", "koi", "koi8", "koi8-r", "koi8_r"],
        legacy_encoder: encoding::all::KOI8_R,
        encoder_impl: Some(encoding_rs::KOI8_R),
    },
    Encoding {
        name: "koi8-u",
        is_multi_byte_encoding: false,
        aliases: &["koi8-ru", "koi8-u"],
        legacy_encoder: encoding::all::KOI8_U,
        encoder_impl: Some(encoding_rs::KOI8_U),
    },
    Encoding {
        name: "macintosh",
        is_multi_byte_encoding: false,
        aliases: &["csmacintosh", "mac", "macintosh", "x-mac-roman"],
        legacy_encoder: encoding::all::MAC_ROMAN,
        encoder_impl: Some(encoding_rs::MACINTOSH),
    },
    Encoding {
        name: "windows-874",
        is_multi_byte_encoding: false,
        aliases: &[
            "dos-874",
            "iso-8859-11",
            "iso8859-11",
            "iso885911",
            "tis-620",
            "windows-874",
        ],
        legacy_encoder: encoding::all::WINDOWS_874,
        encoder_impl: Some(encoding_rs::WINDOWS_874),
    },
    Encoding {
        name: "windows-1250",
        is_multi_byte_encoding: false,
        aliases: &["cp1250", "windows-1250", "x-cp1250"],
        legacy_encoder: encoding::all::WINDOWS_1250,
        encoder_impl: Some(encoding_rs::WINDOWS_1250),
    },
    Encoding {
        name: "windows-1251",
        is_multi_byte_encoding: false,
        aliases: &["cp1251", "windows-1251", "x-cp1251"],
        legacy_encoder: encoding::all::WINDOWS_1251,
        encoder_impl: Some(encoding_rs::WINDOWS_1251),
    },
    Encoding {
        name: "windows-1252",
        is_multi_byte_encoding: false,
        aliases: &[
            "ansi_x3.4-1968",
            "cp1252",
            "cp819",
            "csisolatin1",
            "ibm819",
            "iso-8859-1",
            "iso-ir-100",
            "iso8859-1",
            "iso88591",
            "iso_8859-1",
            "iso_8859-1:1987",
            "l1",
            "latin1",
            "windows-1252",
            "x-cp1252",
            // Note: <https://encoding.spec.whatwg.org/#concept-encoding-get>
            // specifies that ascii is simply an alias for cp1252, but
            // the various detection tests in this crate will fail if
            // we make it a strict alias, so we have a separate ascii
            // Encoding object and do not include the ascii aliases here
            // "ascii",
            // "us-ascii",
        ],
        legacy_encoder: encoding::all::WINDOWS_1252,
        encoder_impl: Some(encoding_rs::WINDOWS_1252),
    },
    Encoding {
        name: "windows-1253",
        is_multi_byte_encoding: false,
        aliases: &["cp1253", "windows-1253", "x-cp1253"],
        legacy_encoder: encoding::all::WINDOWS_1253,
        encoder_impl: Some(encoding_rs::WINDOWS_1253),
    },
    Encoding {
        name: "windows-1254",
        is_multi_byte_encoding: false,
        aliases: &[
            "cp1254",
            "csisolatin5",
            "iso-8859-9",
            "iso-ir-148",
            "iso8859-9",
            "iso88599",
            "iso_8859-9",
            "iso_8859-9:1989",
            "l5",
            "latin5",
            "windows-1254",
            "x-cp1254",
        ],
        legacy_encoder: encoding::all::WINDOWS_1254,
        encoder_impl: Some(encoding_rs::WINDOWS_1254),
    },
    Encoding {
        name: "windows-1255",
        is_multi_byte_encoding: false,
        aliases: &["cp1255", "windows-1255", "x-cp1255"],
        legacy_encoder: encoding::all::WINDOWS_1255,
        encoder_impl: Some(encoding_rs::WINDOWS_1255),
    },
    Encoding {
        name: "windows-1256",
        is_multi_byte_encoding: false,
        aliases: &["cp1256", "windows-1256", "x-cp1256"],
        legacy_encoder: encoding::all::WINDOWS_1256,
        encoder_impl: Some(encoding_rs::WINDOWS_1256),
    },
    Encoding {
        name: "windows-1257",
        is_multi_byte_encoding: false,
        aliases: &["cp1257", "windows-1257", "x-cp1257"],
        legacy_encoder: encoding::all::WINDOWS_1257,
        encoder_impl: Some(encoding_rs::WINDOWS_1257),
    },
    Encoding {
        name: "windows-1258",
        is_multi_byte_encoding: false,
        aliases: &["cp1258", "windows-1258", "x-cp1258"],
        legacy_encoder: encoding::all::WINDOWS_1258,
        encoder_impl: Some(encoding_rs::WINDOWS_1258),
    },
    Encoding {
        name: "x-mac-cyrillic",
        is_multi_byte_encoding: false,
        aliases: &["x-mac-cyrillic", "x-mac-ukrainian"],
        legacy_encoder: encoding::all::MAC_CYRILLIC,
        encoder_impl: Some(encoding_rs::X_MAC_CYRILLIC),
    },
    Encoding {
        name: "gbk",
        is_multi_byte_encoding: true,
        aliases: &[
            "chinese",
            "csgb2312",
            "csiso58gb231280",
            "gb2312",
            "gb_2312",
            "gb_2312-80",
            "gbk",
            "iso-ir-58",
            "x-gbk",
        ],
        legacy_encoder: encoding::all::GBK,
        encoder_impl: Some(encoding_rs::GBK),
    },
    Encoding {
        name: "gb18030",
        is_multi_byte_encoding: true,
        aliases: &["gb18030"],
        legacy_encoder: encoding::all::GB18030,
        encoder_impl: Some(encoding_rs::GB18030),
    },
    Encoding {
        name: "big5",
        is_multi_byte_encoding: true,
        aliases: &["big5", "big5-hkscs", "cn-big5", "csbig5", "x-x-big5"],
        legacy_encoder: encoding::all::BIG5_2003,
        encoder_impl: Some(encoding_rs::BIG5),
    },
    Encoding {
        name: "euc-jp",
        is_multi_byte_encoding: true,
        aliases: &["cseucpkdfmtjapanese", "euc-jp", "x-euc-jp"],
        legacy_encoder: encoding::all::EUC_JP,
        encoder_impl: Some(encoding_rs::EUC_JP),
    },
    Encoding {
        name: "iso-2022-jp",
        is_multi_byte_encoding: true,
        aliases: &["csiso2022jp", "iso-2022-jp"],
        legacy_encoder: encoding::all::ISO_2022_JP,
        encoder_impl: Some(encoding_rs::ISO_2022_JP),
    },
    Encoding {
        name: "shift_jis",
        is_multi_byte_encoding: true,
        aliases: &[
            "csshiftjis",
            "ms932",
            "ms_kanji",
            "shift-jis",
            "shift_jis",
            "sjis",
            "windows-31j",
            "x-sjis",
        ],
        legacy_encoder: encoding::all::WINDOWS_31J,
        encoder_impl: Some(encoding_rs::SHIFT_JIS),
    },
    Encoding {
        name: "euc-kr",
        is_multi_byte_encoding: true,
        aliases: &[
            "cseuckr",
            "csksc56011987",
            "euc-kr",
            "iso-ir-149",
            "korean",
            "ks_c_5601-1987",
            "ks_c_5601-1989",
            "ksc5601",
            "ksc_5601",
            "windows-949",
        ],
        legacy_encoder: encoding::all::WINDOWS_949,
        encoder_impl: Some(encoding_rs::EUC_KR),
    },
    /* Not supported by encoding_rs
    Encoding {
        name: "hz",
        is_multi_byte_encoding: true,
        aliases: &["hz"],
        legacy_encoder: encoding::all::HZ,
        encoder_impl: None, // No implementation
    },
    */
    Encoding {
        name: "utf-16be",
        is_multi_byte_encoding: true,
        aliases: &["unicodefffe", "utf-16be"],
        legacy_encoder: encoding::all::UTF_16BE,
        encoder_impl: Some(encoding_rs::UTF_16BE),
    },
    Encoding {
        name: "utf-16le",
        is_multi_byte_encoding: true,
        aliases: &[
            "csunicode",
            "iso-10646-ucs-2",
            "ucs-2",
            "unicode",
            "unicodefeff",
            "utf-16",
            "utf-16le",
        ],
        legacy_encoder: encoding::all::UTF_16LE,
        encoder_impl: Some(encoding_rs::UTF_16LE),
    },
    Encoding {
        name: "utf-8",
        is_multi_byte_encoding: true,
        aliases: &[
            "unicode-1-1-utf-8",
            "unicode11utf8",
            "unicode20utf8",
            "utf-8",
            "utf8",
            "x-unicode20utf8",
        ],
        legacy_encoder: encoding::all::UTF_8,
        encoder_impl: Some(encoding_rs::UTF_8),
    },
];

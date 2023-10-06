# Charset Normalizer
[![charset-normalizer-rs on docs.rs][docsrs-image]][docsrs]
[![charset-normalizer-rs on crates.io][crates-image]][crates]

[docsrs-image]: https://docs.rs/charset-normalizer-rs/badge.svg
[docsrs]: https://docs.rs/charset-normalizer-rs
[crates-image]: https://img.shields.io/crates/v/charset-normalizer-rs.svg
[crates]: https://crates.io/crates/charset-normalizer-rs/

A library that helps you read text from an unknown charset encoding.<br /> Motivated by original Python version of `charset-normalizer`,
I'm trying to resolve the issue by taking a new approach.
All IANA character set names for which the Rust `encoding` library provides codecs are supported.

This project is port of original Pyhon version of [Charset Normalizer](https://github.com/Ousret/charset_normalizer).
The biggest difference between Python and Rust versions - number of supported encodings as each langauge has own encoding / decoding library.
In Rust version only encoding from [WhatWG standard](https://encoding.spec.whatwg.org) are supported. 
Python version supports more encodings, but a lot of them are old almost unused ones.

## ⚡ Performance

This package offer better performance than Python version (4 times faster, than MYPYC version of charset-normalizer, 8 times faster than usual Python version).
In comparison with `chardet` and `chardetng` packages it has approximately the same speed but more accurate. 
Here are some numbers.

| Package                                                                                     |  Accuracy  | Mean per file (ms) | File per sec (est) |
|---------------------------------------------------------------------------------------------|:----------:|:------------------:|:------------------:|
| [chardet](https://crates.io/crates/chardet)                                                 |   82.6 %   |        3 ms        |    333 file/sec    |
| [chardetng](https://crates.io/crates/chardetng)                                             |   90.7 %   |       1.6 ms       |    625 file/sec    |
| charset-normalizer-rs                                                                       | **97.1 %** |     **1.5 ms**     |    666 file/sec    |
| [charset-normalizer](https://github.com/Ousret/charset_normalizer) (Python + MYPYC version) |  **98 %**  |      **8 ms**      |    125 file/sec    |

| Package                                                                                     | 99th percentile | 95th percentile | 50th percentile |
|---------------------------------------------------------------------------------------------|:---------------:|:---------------:|:---------------:|
| [chardet](https://crates.io/crates/chardet)                                                 |      8 ms       |      2 ms       |     0.2 ms      |
| [chardetng](https://crates.io/crates/chardetng)                                             |      14 ms      |      5 ms       |     0.5 ms      |
| charset-normalizer-rs                                                                       |      12 ms      |      5 ms       |     0.7 ms      |
| [charset-normalizer](https://github.com/Ousret/charset_normalizer) (Python + MYPYC version) |      94 ms      |      37 ms      |      3 ms       |

Stats are generated using 400+ files using default parameters. These results might change at any time. 
The dataset can be updated to include more files. The actual delays heavily depends on your CPU capabilities. 
The factors should remain the same. Rust version dataset has been reduced as number of supported encodings is lower than in Python version.

There is a still possibility to speed up library, so I'll appreciate any contributions.

## ✨ Installation

Library installation:

```console
cargo add charset-normalizer-rs
```

Binary CLI tool installation:
```console
cargo install charset-normalizer-rs
```

## 🚀 Basic Usage

### CLI
This package comes with a CLI, which supposes to be compatible with Python version CLI tool.

```console
normalizer -h
Usage: normalizer [OPTIONS] <FILES>...

Arguments:
  <FILES>...  File(s) to be analysed

Options:
  -v, --verbose                Display complementary information about file if any. Stdout will contain logs about the detection process
  -a, --with-alternative       Output complementary possibilities if any. Top-level JSON WILL be a list
  -n, --normalize              Permit to normalize input file. If not set, program does not write anything
  -m, --minimal                Only output the charset detected to STDOUT. Disabling JSON output
  -r, --replace                Replace file when trying to normalize it instead of creating a new one
  -f, --force                  Replace file without asking if you are sure, use this flag with caution
  -t, --threshold <THRESHOLD>  Define a custom maximum amount of chaos allowed in decoded content. 0. <= chaos <= 1 [default: 0.2]
  -h, --help                   Print help
  -V, --version                Print version
```

```bash
normalizer ./data/sample.1.fr.srt
```

🎉 The CLI produces easily usable stdout result in JSON format (should be the same as in Python version).

```json
{
    "path": "/home/default/projects/charset_normalizer/data/sample.1.fr.srt",
    "encoding": "cp1252",
    "encoding_aliases": [
        "1252",
        "windows_1252"
    ],
    "alternative_encodings": [
        "cp1254",
        "cp1256",
        "cp1258",
        "iso8859_14",
        "iso8859_15",
        "iso8859_16",
        "iso8859_3",
        "iso8859_9",
        "latin_1",
        "mbcs"
    ],
    "language": "French",
    "alphabets": [
        "Basic Latin",
        "Latin-1 Supplement"
    ],
    "has_sig_or_bom": false,
    "chaos": 0.149,
    "coherence": 97.152,
    "unicode_path": null,
    "is_preferred": true
}
```

### Rust

Library offers two main methods. First one is `from_bytes`, which processes text using bytes as input parameter:
```rust
use charset_normalizer_rs::from_bytes;

fn test_from_bytes() {
    let result = from_bytes(&vec![0x84, 0x31, 0x95, 0x33], None);
    let best_guess = result.get_best();
    assert_eq!(
        best_guess.unwrap().encoding(),
        "gb18030",
    );
}
test_from_bytes();
```

`from_path` processes text using filename as input parameter:
```rust
use std::path::PathBuf;
use charset_normalizer_rs::from_path;

fn test_from_path() {
    let result = from_path(&PathBuf::from("src/tests/data/samples/sample-chinese.txt"), None).unwrap();
    let best_guess = result.get_best();
    assert_eq!(
        best_guess.unwrap().encoding(),
        "big5",
    );
}
test_from_path();
```

## 😇 Why

When I started using Chardet (Python version), I noticed that it was not suited to my expectations, and I wanted to propose a
reliable alternative using a completely different method. Also! I never back down on a good challenge!

I **don't care** about the **originating charset** encoding, because **two different tables** can
produce **two identical rendered string.**
What I want is to get readable text, the best I can. 

In a way, **I'm brute forcing text decoding.** How cool is that? 😎

## 🍰 How

  - Discard all charset encoding table that could not fit the binary content.
  - Measure noise, or the mess once opened (by chunks) with a corresponding charset encoding.
  - Extract matches with the lowest mess detected.
  - Additionally, we measure coherence / probe for a language.

**Wait a minute**, what is noise/mess and coherence according to **YOU?**

*Noise :* I opened hundred of text files, **written by humans**, with the wrong encoding table. **I observed**, then
**I established** some ground rules about **what is obvious** when **it seems like** a mess.
 I know that my interpretation of what is noise is probably incomplete, feel free to contribute in order to
 improve or rewrite it.

*Coherence :* For each language there is on earth, we have computed ranked letter appearance occurrences (the best we can). So I thought
that intel is worth something here. So I use those records against decoded text to check if I can detect intelligent design.

## ⚡ Known limitations

  - Language detection is unreliable when text contains two or more languages sharing identical letters. (eg. HTML (english tags) + Turkish content (Sharing Latin characters))
  - Every charset detector heavily depends on sufficient content. In common cases, do not bother run detection on very tiny content.

## 👤 Contributing

Contributions, issues and feature requests are very much welcome.<br />
Feel free to check [issues page](https://github.com/nickspring/charset-normalizer-rs/issues) if you want to contribute.

## 📝 License

Copyright © [Nikolay Yarovoy @nickspring](https://github.com/nickspring) - porting to Rust. <br />
Copyright © [Ahmed TAHRI @Ousret](https://github.com/Ousret) - original Python version and some parts of this document.<br />
This project is [MIT](https://github.com/nickspring/charset-normalizer-rs/blob/master/LICENSE) licensed.

Characters frequencies used in this project © 2012 [Denny Vrandečić](http://simia.net/letters/)


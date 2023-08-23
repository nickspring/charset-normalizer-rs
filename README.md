<h1 align="center">Charset Detection, for Everyone 👋</h1>

<p align="center">
  <sup>The Real First Universal Charset Detector, Rust version</sup><br>
</p>

> A library that helps you read text from an unknown charset encoding.<br /> Motivated by original Python version of `charset-normalizer`,
> I'm trying to resolve the issue by taking a new approach.
> All IANA character set names for which the Rust `encoding` library provides codecs are supported.

This project is port of original Pyhon version of [Charset Normalizer](https://github.com/Ousret/charset_normalizer).
The biggest difference between Python and Rust versions - number of supported encodings as each langauge has own encoding / decoding library.
In Rust version only encoding from [WhatWG standard](https://encoding.spec.whatwg.org) are supported. 
Python version supports more encodings, but a lot of them are old almost unused ones.

| Feature                                          | [Chardet](https://github.com/chardet/chardet) |                                           Charset Normalizer                                           | [cChardet](https://github.com/PyYoshi/cChardet) |
|--------------------------------------------------|:---------------------------------------------:|:------------------------------------------------------------------------------------------------------:|:-----------------------------------------------:|
| `Fast`                                           |                     ❌<br>                     |                                                 ✅<br>                                                  |                     ✅ <br>                      |
| `Universal**`                                    |                       ❌                       |                                                   ✅                                                    |                        ❌                        |
| `Reliable` **without** distinguishable standards |                       ❌                       |                                                   ✅                                                    |                        ✅                        |
| `Reliable` **with** distinguishable standards    |                       ✅                       |                                                   ✅                                                    |                        ✅                        |
| `License`                                        |           LGPL-2.1<br>_restrictive_           |                                                  MIT                                                   |            MPL-1.1<br>_restrictive_             |
| `Native Python`                                  |                       ✅                       |                                                   ✅                                                    |                        ❌                        |
| `Detect spoken language`                         |                       ❌                       |                                                   ✅                                                    |                       N/A                       |
| `UnicodeDecodeError Safety`                      |                       ❌                       |                                                   ✅                                                    |                        ❌                        |
| `Whl Size`                                       |                   193.6 kB                    |                                                 40 kB                                                  |                     ~200 kB                     |
| `Supported Encoding`                             |                      33                       | 🎉 [90](https://charset-normalizer.readthedocs.io/en/latest/user/support.html#supported-encodings) |                       40                        |

*\*\* : They are clearly using specific code for a specific encoding even if covering most of used one*<br> 

## ⚡ Performance

This package offer better performance than its counterpart Chardet. Here are some numbers.

| Package                                       | Accuracy | Mean per file (ms) | File per sec (est) |
|-----------------------------------------------|:--------:|:------------------:|:------------------:|
| [chardet](https://github.com/chardet/chardet) |   86 %   |       200 ms       |     5 file/sec     |
| charset-normalizer                            | **98 %** |     **10 ms**      |    100 file/sec    |

| Package                                       | 99th percentile | 95th percentile | 50th percentile |
|-----------------------------------------------|:---------------:|:---------------:|:---------------:|
| [chardet](https://github.com/chardet/chardet) |     1200 ms     |     287 ms      |      23 ms      |
| charset-normalizer                            |     100 ms      |      50 ms      |      5 ms       |

Chardet's performance on larger file (1MB+) can be very poor. Expect huge difference on large payload.

> Stats are generated using 400+ files using default parameters. These results might change at any time. 
> The dataset can be updated to include more files. The actual delays heavily depends on your CPU capabilities. 
> The factors should remain the same.
> Rust version dataset has been reduced as number of supported encodings is lower than in Python version.

## ✨ Installation

Using pip:

```sh
cargo add charset-normalizer-rs
```

## 🚀 Basic Usage

### CLI
This package comes with a CLI, which supposes to be compatible with Python version CLI tool.

```
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

### Python
*Just print out normalized text*
```python
from charset_normalizer import from_path

results = from_path('./my_subtitle.srt')

print(str(results.best()))
```

*Upgrade your code without effort*
```python
from charset_normalizer import detect
```

The above code will behave the same as **chardet**. We ensure that we offer the best (reasonable) BC result possible.

See the docs for advanced usage : [readthedocs.io](https://charset-normalizer.readthedocs.io/en/latest/)

## 😇 Why

When I started using Chardet (Python version), I noticed that it was not suited to my expectations, and I wanted to propose a
reliable alternative using a completely different method. Also! I never back down on a good challenge!

I **don't care** about the **originating charset** encoding, because **two different tables** can
produce **two identical rendered string.**
What I want is to get readable text, the best I can. 

In a way, **I'm brute forcing text decoding.** How cool is that ? 😎

## 🍰 How

  - Discard all charset encoding table that could not fit the binary content.
  - Measure noise, or the mess once opened (by chunks) with a corresponding charset encoding.
  - Extract matches with the lowest mess detected.
  - Additionally, we measure coherence / probe for a language.

**Wait a minute**, what is noise/mess and coherence according to **YOU ?**

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

Copyright © [Nikolay Yarovoy @nickspring](https://github.com/nickspring) - porting to Rust.
Copyright © [Ahmed TAHRI @Ousret](https://github.com/Ousret) - original Python version and some parts of this document.<br />
This project is [MIT](https://github.com/nickspring/charset-normalizer-rs/blob/master/LICENSE) licensed.

Characters frequencies used in this project © 2012 [Denny Vrandečić](http://simia.net/letters/)


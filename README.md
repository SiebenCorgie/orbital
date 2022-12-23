<div align="center">

# 🪐 Orbital 🪐

A cosmic, polyphonic, additive FM synthesizer. 

[![dependency status](https://deps.rs/repo/gitlab/tendsinmende/orbital/status.svg)](https://deps.rs/repo/gitlab/tendsinmende/orbital)
[![ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/L3L3F09W2)

![Banner](res/banner.gif "Orbital")
</div>

## Features
- Relative (to the oscillator frequency) and absolute frequency modulation: This allows you to either model interesting absolute soundscapes, or soundscapes that change relative to the currently played key.
- Polyphony: Up to 10 concurrent voices.
- Sample correct midi event offsets
- 5 stage ADSR Midi-filter including: delay, attack, hold, decay, sustain level, release.
- High quality sin based oscillators

## Planed features
- Better voice addressing
- Multiple voice composition options: currently all voices are mixed via a sigmoid which produces distortion when playing multiple high volume voices.
- SIMD implementation: Currently all oscillators in a bank are processed sequentially using sine waves. This produces a high quality result. The CPU load hover could be reduced dramatically if SIMD and, for lower frequencies a sine approximation was used.
- Phase modulation and amplitude: Maybe let the user chose the type of modulation on a *per planet* basis.
- UI improvements
## Getting the plugin

There are two ways: Either you use the build instructions below, or you write me on one of the platforms mentioned on [siebencorgie.rs](https://siebencorgie.rs) and I'll try to send you a recent version when I have time.

## Building 
To build, install a [Rust toolchain and Cargo](https://www.rust-lang.org/). After that issue the following command in a terminal:

``` shell
cargo xtask bundle orbital --release
```

This will build the VST3 and Clap version of the plugin in `target/bundled`.

Now copy the desired plugin somewhere your DAW can find it.


## License

The whole project is licensed under MPL v2.0, all contributions will be licensed the same. Have a look at Mozilla's [FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/) to see if this fits your use-case.

If you use the VST3 plugin, note the following (from [nih-plug](https://github.com/robbert-vdh/nih-plug)):

> However, the VST3 bindings used by nih_export_vst3!() are licensed under the GPLv3 license. This means that unless you replace these bindings with your own bindings made from scratch, any VST3 plugins built with NIH-plug need to be able to comply with the terms of the GPLv3 license.



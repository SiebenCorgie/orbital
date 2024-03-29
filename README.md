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
- 5 stage ADSR Midi-filter including: delay, attack, hold, decay, sustain level and release.
- High quality cos based oscillators
- Two voice composition options: Linear (clear for less voices), Sigmoid (Warm distortion for more voices).
- SIMD oscillator implementation using [portable-simd](https://github.com/rust-lang/portable-simd) and [sleef-rs](https://crates.io/crates/sleef).

## Getting the plugin

For every release, there are binary builds for Window, Linux and OSX (as VST3 and Clap plugins) available on the [release](https://github.com/SiebenCorgie/orbital/releases) page.

## Building 
To build, install a [Rust toolchain and Cargo](https://www.rust-lang.org/). Make sure you are using the `nightly` toolchain.

After that issue the following command in a terminal:
``` shell
cargo xtask bundle orbital --release
```

This will build the VST3 and Clap version of the plugin in `target/bundled`.

Now copy the desired plugin somewhere your DAW can find it.

## Quick start guide:

<div align="center">


[![Quick Start Video](https://img.youtube.com/vi/KKwYsVGxOgs/0.jpg)](http://www.youtube.com/watch?v=KKwYsVGxOgs)


</div>

The synthesizer is fundamentally a frequency modulation based synthesizer. Each orbit represents a oscillator with a given speed and possibly a parent that is modulated with that frequency. If the orbit is around the sun, the orbit is considered "primary". This means that the frequency of the oscillator is a fundamental frequency of the resulting sound.

You add a primary frequency by clicking somewhere into the void around the sun. Note that you can change the orbit height and the location of the planet by dragging one of those with your pointer.
You can add a child modulator to a planet by dragging the edge of the planet out. The orbits height of a child changes the influence to the parent.
The speed of an oscillator is (octave wise) increased or decreased when scrolling while hovering over its orbit.

This is the main interface to the synth. However, apart from a standard [ADSR](https://www.wikiaudio.org/adsr-envelope/) filter three parameters at the top are interesting. The most left parameter changes the relation between a modulator and its parent. When set to *relative* the modulator takes the parents frequency and changes that. When set to *absolute* the reference frequency will always be 440 Hz.
The next parameter changes how voices are mixed. Linear takes the *ground truth* output and clamps it to -1.0 - 1.0. [Sigmoid](https://en.wikipedia.org/wiki/Sigmoid_function) uses a function to map any value in that range.

Finally you can enable *reset phase*, which will reset the internal oscillator before playing a new note. This is interesting if you want to be sure that successive played notes sound the same.

## License

The whole project is licensed under MPL v2.0, all contributions will be licensed the same. Have a look at Mozilla's [FAQ](https://www.mozilla.org/en-US/MPL/2.0/FAQ/) to see if this fits your use-case.

If you use the VST3 plugin, note the following (from [nih-plug](https://github.com/robbert-vdh/nih-plug)):

> However, the VST3 bindings used by nih_export_vst3!() are licensed under the GPLv3 license. This means that unless you replace these bindings with your own bindings made from scratch, any VST3 plugins built with NIH-plug need to be able to comply with the terms of the GPLv3 license.



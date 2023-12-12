# Rubato Downsampler

## Building

After installing [Rust](https://rustup.rs/), you can compile Rubato Downsampler as follows:

```shell
cargo xtask bundle rubato_downsampler --release
```

## Install

### macOS

```shell
rsync -ahv --delete target/bundled/Rubato\ Downsampler.clap/ ~/Library/Audio/Plug-Ins/CLAP/Rubato\ Downsampler.clap
rsync -ahv --delete target/bundled/Rubato\ Downsampler.vst3/ ~/Library/Audio/Plug-Ins/VST3/Rubato\ Downsampler.vst3
```

## Validation

### CLAP

```shell
clap-validator validate target/bundled/Rubato\ Downsampler.clap
```

### VST3

```shell
pluginval --verbose --strictness-level 5 target/bundled/Rubato\ Downsampler.vst3
```

# Rubato Downsampler

## 既知の制限

DAWのバッファーサイズは128サンプルである必要があります。

## Building

After installing [Rust](https://rustup.rs/), you can compile Rubato Downsampler as follows:

```shell
cargo xtask bundle rubato_downsampler --release
```

## macOS universal binary

```shell
cargo xtask bundle-universal rubato_downsampler --release
```

## Install

### macOS

```shell
PLUGIN_NAME="Rubato Downsampler"; rsync -ahv --delete target/bundled/${PLUGIN_NAME}.clap/ ~/Library/Audio/Plug-Ins/CLAP/${PLUGIN_NAME}.clap; rsync -ahv --delete target/bundled/${PLUGIN_NAME}.vst3/ ~/Library/Audio/Plug-Ins/VST3/${PLUGIN_NAME}.vst3
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

## Debug

### Mac

#### AudioPluginHost.app of JUCE

Install JUCE and build AudioPluginHost.app  

```shell
lldb /Applications/JUCE/extras/AudioPluginHost/Builds/MacOSX/build/Release/AudioPluginHost.app/Contents/MacOS/AudioPluginHost
(lldb) run
```

Then, scan VST3 plugins and test them.  

#### Reaper

Install [REAPER](https://www.reaper.fm/).  

```shell
lldb /Applications/REAPER.app/Contents/MacOS/REAPER
(lldb) run
```

Then, scan VST3 or Clap plugins and test them.  

#### Cycling'74 / Max

Install [Max](https://cycling74.com/).  

```shell
lldb /Applications/Max.app/Contents/MacOS/Max
(lldb) run
```

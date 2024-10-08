use nih_plug::prelude::*;
use rubato::{FastFixedIn, FastFixedOut, PolynomialDegree, Resampler};
use std::{cmp::max, sync::Arc, vec};

/// プラグインがサポートする最高サンプリングレート Hz
const MAX_SAMPLE_RATE: f64 = 192_000.0;
/// OTO Biscuitの最低リサンプリングレート Hz
const MIN_RESAMPLE_RATE: f64 = 250.0;
/// OTO Biscuitの最高リサンプリングレート Hz
const MAX_RESAMPLE_RATE: f64 = 30_000.0;
/// リサンプリングの最大比率
const MAX_RESAMPLE_RATIO_RELATIVE: f64 = (MAX_SAMPLE_RATE + 10_000f64) / MIN_RESAMPLE_RATE;
/// リサンプリング処理のチャンクサイズ サンプル
const RESAMPLE_CHUNK_SIZE: usize = 128;

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

struct RubatoDownsampler {
    params: Arc<RubatoDownsamplerParams>,
    /// ダウンサンプリングのためのresampler
    resampler_in: FastFixedIn<f32>,
    temp_buffer: Vec<Vec<f32>>,
    /// DAWのサンプルレートに戻すためのresampler
    resampler_out: FastFixedOut<f32>,
    sample_rate: f32,
    /// 直近のリサンプリングレート リサンプルレートの変更を検知するために使用
    last_resample_rate: i32,
}

#[derive(Params)]
struct RubatoDownsamplerParams {
    /// The parameter's ID is used to identify the parameter in the wrapped plugin API. As long as
    /// these IDs remain constant, you can rename and reorder these fields as you wish. The
    /// parameters are exposed to the host in the same order they were defined. In this case, this
    /// gain parameter is stored as linear gain while the values are displayed in decibels.
    #[id = "Resample"]
    pub resample: IntParam,
}

impl Default for RubatoDownsampler {
    fn default() -> Self {
        Self {
            params: Arc::new(RubatoDownsamplerParams::default()),
            resampler_in: FastFixedIn::new(
                1.0f64,
                MAX_RESAMPLE_RATIO_RELATIVE,
                PolynomialDegree::Cubic,
                RESAMPLE_CHUNK_SIZE,
                2,
            )
            .unwrap(),
            temp_buffer: vec![vec![0.0; RESAMPLE_CHUNK_SIZE]; 2],
            resampler_out: FastFixedOut::new(
                1.0f64,
                MAX_RESAMPLE_RATIO_RELATIVE,
                PolynomialDegree::Cubic,
                RESAMPLE_CHUNK_SIZE,
                2,
            )
            .unwrap(),
            sample_rate: 0.0,
            last_resample_rate: 0,
        }
    }
}

impl Default for RubatoDownsamplerParams {
    fn default() -> Self {
        Self {
            resample: IntParam::new(
                "Resample",
                10_000,
                IntRange::Linear {
                    min: MIN_RESAMPLE_RATE as i32,
                    max: MAX_RESAMPLE_RATE as i32,
                },
            )
            .with_unit("Hz"),
        }
    }
}

impl RubatoDownsampler {
    /// 現在のサンプルレートとCHUNK_SIZEから実行可能なリサンプリングレートを探す
    ///
    /// # Arguments
    ///
    /// * `resample_rate` - リサンプリングレート Hz
    ///
    /// # Returns
    ///
    /// 実行可能なリサンプリングレート Hz
    fn find_resample_rate(&self, resample_rate: i32) -> i32 {
        let mut rate = resample_rate;
        while (RESAMPLE_CHUNK_SIZE * rate as usize) % (self.sample_rate as usize) != 0 {
            rate += 1;
        }
        rate
    }

    /// リサンプリングレートを更新する.
    ///
    /// resampler_inでダウンサンプリングを行い、resampler_outで元のサンプルレートに戻すように設定する.
    /// ブロック処理する都合上、全てのサンプルレートにダウンサンプリングできるわけではない。設定できるサンプルレートは、find_resample_rate()で探索する必要がある.
    ///
    /// # Arguments
    /// * `resample_rate` - 何Hzにダウンサンプリングするか    
    fn update_resample_rate(&mut self, resample_rate: i32) {
        let resample_ratio = resample_rate as f64 / self.sample_rate as f64;
        nih_log!("Resample ratio: {}", resample_ratio);
        self.resampler_in
            .set_resample_ratio(resample_ratio, false)
            .expect("Failed to set resample ratio to resampler_in");
        // 元のサンプルレートに戻すために逆数を取る
        self.resampler_out
            .set_resample_ratio(resample_ratio.recip(), false)
            .expect("Failed to set resample ratio to resampler_out");
    }
}

impl Plugin for RubatoDownsampler {
    const NAME: &'static str = "Rubato Downsampler";
    const VENDOR: &'static str = "Akiyuki Okayasu";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "akiyuki.okayasu@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        // Individual ports and the layout as a whole can be named here. By default these names
        // are generated as needed. This layout will be called 'Stereo', while a layout with
        // only one input and output channel would be called 'Mono'.
        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        nih_log!("Initializing plugin");
        self.sample_rate = buffer_config.sample_rate;
        true
    }

    fn reset(&mut self) {
        nih_log!("Resetting plugin");
        self.resampler_in.reset();
        self.resampler_out.reset();

        let resample_rate = self.params.resample.value();
        self.last_resample_rate = resample_rate;
        nih_log!("Target resample rate: {} Hz", resample_rate);

        // 現在のサンプルレートとCHUNK_SIZEから実行可能なリサンプリングレートを探索
        let resample_rate = self.find_resample_rate(resample_rate);
        nih_log!("Find resample rate: {} Hz", resample_rate);
        self.update_resample_rate(resample_rate);

        // リサンプリングの一時バッファーに必要となる最大のフレーム数を取得
        let max_frames = max(
            self.resampler_in.output_frames_max(),
            self.resampler_out.input_frames_max(),
        );
        // リサンプリングの一時バッファーをリサイズ
        if self.temp_buffer[0].len() < max_frames {
            self.temp_buffer
                .iter_mut()
                .for_each(|e| e.resize(max_frames, 0.0));
        }

        // リサンプリング処理の遅延をprint
        let delay = self.resampler_in.output_delay();
        nih_log!("Resampler_in delay: {}", delay);
        let delay = self.resampler_out.output_delay();
        nih_log!("Resampler_out delay: {}", delay);

        let frames = self.resampler_in.input_frames_next();
        nih_log!("Resampler_in frames: {}", frames);
        let frames = self.resampler_out.input_frames_next();
        nih_log!("Resampler_out frames: {}", frames);
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let resample_rate = self.params.resample.value();
        if self.last_resample_rate != resample_rate {
            nih_log!("Target resample rate: {} Hz", resample_rate);
            self.last_resample_rate = resample_rate;

            // 現在のサンプルレートとCHUNK_SIZEから実行可能なリサンプリングレートを探索
            let resample_rate = self.find_resample_rate(resample_rate);
            nih_log!("Find resample rate: {} Hz", resample_rate);
            self.update_resample_rate(resample_rate);

            // リサンプリング処理の遅延をprint
            let delay = self.resampler_in.output_delay();
            nih_log!("Resampler_in delay: {}", delay);
            let delay = self.resampler_out.output_delay();
            nih_log!("Resampler_out delay: {}", delay);
        }

        let buf: &mut [&mut [f32]] = buffer.as_slice();
        let temp = self.temp_buffer.as_mut_slice();
        self.resampler_in
            .process_into_buffer(buf, temp, None)
            .expect("Failed to resample_in");
        self.resampler_out
            .process_into_buffer(temp, buf, None)
            .expect("Failed to resample_out");

        ProcessStatus::Normal
    }
}

impl ClapPlugin for RubatoDownsampler {
    const CLAP_ID: &'static str = "com.akiyukiokayasu.rubato-downsampler";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A short description of your plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for RubatoDownsampler {
    const VST3_CLASS_ID: [u8; 16] = *b"rubatodownsample";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_clap!(RubatoDownsampler);
nih_export_vst3!(RubatoDownsampler);

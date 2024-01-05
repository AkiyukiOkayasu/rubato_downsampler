use nih_plug::prelude::*;
use rubato::{FastFixedIn, FastFixedOut, PolynomialDegree, Resampler};
use std::{sync::Arc, vec};

const MAX_SAMPLE_RATE: f64 = 192_000.0;
const MIN_RESAMPLE_RATE: f64 = 250.0; //OTO Biscuitの最低リサンプリングレート Hz
const MAX_RESAMPLE_RATE: f64 = 30_000.0; //OTO Biscuitの最高リサンプリングレート Hz
const MAX_RESAMPLE_RATIO_RELATIVE: f64 = MAX_SAMPLE_RATE / MIN_RESAMPLE_RATE;
const RESAMPLE_CHUNK_SIZE: usize = 128;

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

struct RubatoDownsampler {
    params: Arc<RubatoDownsamplerParams>,
    resampler_in: FastFixedIn<f32>,
    temp_buffer: Vec<Vec<f32>>,
    resampler_out: FastFixedOut<f32>,
    sample_rate: f32,
    resample_ratio: f64,
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
                PolynomialDegree::Linear,
                RESAMPLE_CHUNK_SIZE,
                2,
            )
            .unwrap(),
            temp_buffer: vec![vec![0.0; RESAMPLE_CHUNK_SIZE]; 2],
            resampler_out: FastFixedOut::new(
                1.0f64,
                MAX_RESAMPLE_RATIO_RELATIVE,
                PolynomialDegree::Linear,
                128,
                2,
            )
            .unwrap(),
            sample_rate: 0.0,
            resample_ratio: 1.0,
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

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

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
        self.sample_rate = buffer_config.sample_rate;
        let resample_rate = self.params.resample.value();
        self.resample_ratio = self.sample_rate as f64 / resample_rate as f64;
        self.resampler_in
            .set_resample_ratio(self.resample_ratio, false)
            .expect("Failed to set resample ratio to resampler_in");
        self.resampler_out
            .set_resample_ratio(self.resample_ratio.recip(), false)
            .expect("Failed to set resample ratio to resampler_out");

        true
    }

    fn reset(&mut self) {
        nih_log!("Resetting plugin");

        self.resampler_in.reset();
        self.resampler_out.reset();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let resample_rate = self.params.resample.value();
        let resample_ratio = resample_rate as f64 / self.sample_rate as f64;
        if self.resample_ratio.round() as i32 != resample_ratio.round() as i32 {
            self.resample_ratio = resample_ratio;
            //TODO 関数化
            self.resampler_in
                .set_resample_ratio(resample_ratio, false)
                .expect("Failed to set resample ratio to resampler_in");
            self.resampler_out
                .set_resample_ratio(resample_ratio.recip(), false)
                .expect("Failed to set resample ratio to resampler_out");
        }

        let buf = buffer.as_slice();
        let temp = self.temp_buffer.as_mut_slice();
        self.resampler_in
            .process_into_buffer(&buf, temp, None)
            .expect("Failed to resample_in");
        self.resampler_out
            .process_into_buffer(&temp, buf, None)
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

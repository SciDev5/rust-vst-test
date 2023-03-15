use std::{array};

use nih_plug::{prelude::{Smoother, SmoothingStyle, BufferConfig}, util};

use crate::{wavetable::{Wavetable}, common_data::CommonDataRef};

struct ToSynthData<
    'a
> {
    wavetable: &'a Wavetable,
    aftertouch: f32,
}

struct Voice {
    sample_rate: f32,
    phase: f32,

    n: i32,
    freq_off: f32,
    freq: f32,
    gain: f32,
}
impl Voice {
    fn new(n: i32) -> Self {
        Self {
            sample_rate: 1.0,
            phase: 0.0,
            n,
            freq: 0.0,
            freq_off: 2.0f32.powf((rand::random::<f32>()-0.5)/6.0*0.01),
            gain: 0.0,
        }
    }
    fn wave(phase: f32, d: ToSynthData) -> f32 {
        // const WAVEFORM:[f32;5] = [1.0,0.5,0.25,0.125,0.0625];
        // let mut sum = 0.0;
        // for (i,k) in WAVEFORM.iter().enumerate() {
        //     sum += k * ((i as f32 + 1.0) * phase * std::f32::consts::TAU).sin()
        // }
        // return sum;
        // if phase > 0.5 {
        //     1.0
        // } else {
        //     -1.0
        // }
        // (std::f32::consts::TAU * phase).sin()

        // let mut sum = (std::f32::consts::TAU * phase).sin();
        // for i in 1..8 {
        //     sum += d.aftertouch.powi(2*i + 1) * (i as f32 * phase * std::f32::consts::TAU).sin()
        // }
        // return sum;

        d.wavetable.sample(phase, d.aftertouch)
    }

    pub fn next(&mut self, d:ToSynthData) -> f32 {
        self.phase += self.freq / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        Voice::wave(self.phase, d) * self.gain
    }

    pub fn update_freq_gain(
        &mut self,
        center_freq: f32,
        detune_per_n: f32,
        noncenter_gain: f32,
        falloff_per_n: f32,
    ) {
        self.freq = center_freq * detune_per_n.powi(self.n) * self.freq_off;
        self.gain = if self.n == 0 {
            1.0
        } else {
            noncenter_gain * falloff_per_n.powi(self.n.abs())
        };
    }
    pub fn init(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;
    }
    pub fn reset(&mut self) {
        self.phase = 0.0;
        self.freq = 0.0;
        self.gain = 0.0;
    }
}

pub enum NoteState {
    DISABLED,
    HELD,
    RELEASED,
}

const N_VOICES: usize = 7;

pub struct NotePlayer {
    sample_rate: f32,

    voice_id: i32,
    channel: u8,
    note_id: u8,

    state: NoteState,
    t_since_start: u32,
    t_since_end: u32,

    data: CommonDataRef,
    voices: [[Voice; N_VOICES]; 2],
    freq_base: f32,

    gain: Smoother<f32>,
    aftertouch: Smoother<f32>,
}

impl NotePlayer {
    pub fn new(
        data: CommonDataRef
    ) -> Self {
        Self {
            sample_rate: 1.0,
            voice_id: 0,
            channel: 0,
            note_id: 0,
            voices: array::from_fn(|_| {
                let k = (N_VOICES as i32 + 1) / 2;
                array::from_fn(|i_raw| {
                    let i = i_raw as i32;
                    let n = if i < k { i } else { i + 1 - 2 * k };
                    Voice::new(n)
                })
            }),
            freq_base: 0.0,
            gain: Smoother::new(SmoothingStyle::Linear(5.0)),
            aftertouch: Smoother::new(SmoothingStyle::Linear(5.0)),

            state: NoteState::DISABLED,
            t_since_start: 0,
            t_since_end: 0,

            data,
        }
    }
    fn increment_t_since(&mut self) {
        match self.state {
            NoteState::DISABLED => {}
            NoteState::HELD => {
                self.t_since_start += 1;
            }
            NoteState::RELEASED => {
                self.t_since_end += 1;
            }
        }
    }
    fn wave(&mut self) -> [f32; 2] {
        let aftertouch = self.aftertouch.next();
        array::from_fn(|channel_id| {
            let mut sum = 0.0f32;
            let wavetable = &self.data.lock().unwrap().wavetable;
            for voice in &mut self.voices[channel_id] {
                sum += voice.next(ToSynthData {
                    aftertouch,
                    wavetable,
                });
            }
            sum
        })
    }
    fn for_each_voice<V>(&mut self, mut cb: V)
    where
        V: FnMut(&mut Voice) -> (),
    {
        for voices_for_channel in &mut self.voices {
            for voice in voices_for_channel {
                cb(voice);
            }
        }
    }
    fn update_voices(&mut self) {
        let freq = self.freq_base;
        self.for_each_voice(|voice| {
            voice.update_freq_gain(freq, 1.01, 0.3, 0.5);
        });
    }

    pub fn next(&mut self) -> [f32; 2] {
        self.increment_t_since();
        self.update_voices();
        let gain = self.gain.next();
        return self.wave().map(|v| v * gain);
    }
    pub fn init(&mut self, buffer_config: &BufferConfig) {
        let sample_rate = buffer_config.sample_rate;
        self.sample_rate = sample_rate;
        self.for_each_voice(|voice| voice.init(sample_rate));
    }
    pub fn reset(&mut self) {
        self.voice_id = 0;
        self.note_id = 0;
        self.freq_base = 0.0;

        self.state = NoteState::DISABLED;
        self.t_since_start = 0;
        self.t_since_end = 0;

        self.gain.reset(0.0);
        self.aftertouch.reset(0.0);

        self.for_each_voice(Voice::reset);
    }

    pub fn trigger(&mut self, channel: u8, voice_id: i32, midi_note_id: u8, velocity: f32) {
        self.state = NoteState::HELD;
        self.t_since_start = 0;

        self.voice_id = voice_id;
        self.channel = channel;
        self.note_id = midi_note_id;

        self.freq_base = util::midi_note_to_freq(midi_note_id);

        self.gain.set_target(self.sample_rate, velocity); // temp
        self.aftertouch.reset(0.0);
    }
    pub fn release(&mut self) {
        self.state = NoteState::RELEASED;
        self.t_since_end = 0;

        self.gain.set_target(self.sample_rate, 0.0); // temp
    }
    pub fn pressure(&mut self, pressure: f32) {
        self.aftertouch.set_target(self.sample_rate, pressure); // temp
    }
    pub fn tuning(&mut self, tuning: f32) {
        self.freq_base = util::f32_midi_note_to_freq(self.note_id as f32 + tuning);
    }

    /** Find the voice to begin a new note on. */
    pub fn find_to_trigger<const N: usize>(noteplayers: &mut [NotePlayer; N]) -> &mut NotePlayer {
        if N == 0 {
            panic!("no voices to pick from in Voice::find_to_start");
        }
        let mut selected: Option<&mut NotePlayer> = None;
        for checking in noteplayers {
            selected = Some(if let Some(selected) = selected {
                match selected.state {
                    NoteState::HELD => {
                        match checking.state {
                            NoteState::DISABLED => checking, // prefer to override unused voice
                            NoteState::RELEASED => checking, // prefer to override released voice over held
                            NoteState::HELD => {
                                // prefer to override the one that started longer ago
                                if checking.t_since_start > selected.t_since_start {
                                    checking
                                } else {
                                    selected
                                }
                            }
                        }
                    }
                    NoteState::RELEASED => {
                        match checking.state {
                            NoteState::DISABLED => checking, // prefer to override unused voice
                            NoteState::HELD => selected, // prefer to override released voice over held
                            NoteState::RELEASED => {
                                // prefer to override the one that ended longer ago
                                if checking.t_since_end > selected.t_since_end {
                                    checking
                                } else {
                                    selected
                                }
                            }
                        }
                    }
                    NoteState::DISABLED => selected, // prefer to override unused voice
                }
            } else {
                checking
            })
        }
        selected.unwrap()
    }

    pub fn find_by_held_note<const N: usize>(
        noteplayers: &mut [NotePlayer; N],
        midi_note_id: u8,
    ) -> Option<&mut NotePlayer> {
        if N == 0 {
            panic!("no voices to pick from in Voice::find_by_note");
        }

        for noteplayer in noteplayers {
            match noteplayer.state {
                NoteState::HELD if noteplayer.note_id == midi_note_id => {
                    return Some(noteplayer);
                }
                _ => (),
            }
        }
        return None;
    }

    /*
    pub fn find_by_voice_id<const N: usize>(
        noteplayers: &mut [NotePlayer; N],
        voice_id: i32,
    ) -> Option<&mut NotePlayer> {
        if N == 0 {
            panic!("no voices to pick from in Voice::find_by_voice_id");
        }
        for noteplayer in noteplayers {
            match noteplayer.state {
                NoteState::RELEASED | NoteState::HELD if noteplayer.voice_id == voice_id => {
                    return Some(noteplayer);
                }
                _ => (),
            }
        }
        return None;
    }
    */
    
    pub fn find_all_by_channel<const N: usize>(
        noteplayers: &mut [NotePlayer; N],
        channel: u8,
    ) -> Vec<&mut NotePlayer> {
        if N == 0 {
            panic!("no voices to pick from in Voice::find_by_voice_id");
        }
        let mut found:Vec<&mut NotePlayer> = Vec::new();
        for noteplayer in noteplayers {
            match noteplayer.state {
                NoteState::RELEASED | NoteState::HELD if noteplayer.channel == channel => {
                    found.push(noteplayer);
                }
                _ => (),
            }
        }
        return found;
    }
}

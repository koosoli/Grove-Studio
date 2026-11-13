//! Small, generated ambience loops. No downloaded recordings or engine assets.
use macroquad::audio::{
    load_sound_from_bytes, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound,
};

pub struct Ambience {
    wind: Option<Sound>,
    day: Option<Sound>,
    night: Option<Sound>,
    step: Option<Sound>,
    playing: bool,
    enabled: bool,
    volume: f32,
    walked: f32,
}
impl Ambience {
    pub async fn new() -> Self {
        Self {
            wind: load_sound_from_bytes(&wave(0)).await.ok(),
            day: load_sound_from_bytes(&wave(1)).await.ok(),
            night: load_sound_from_bytes(&wave(2)).await.ok(),
            step: load_sound_from_bytes(&wave(3)).await.ok(),
            playing: false,
            enabled: false,
            volume: 0.,
            walked: 0.,
        }
    }
    pub fn update(&mut self, enabled: bool, volume: f32, wind: f32, hours: f32) {
        self.enabled = enabled;
        self.volume = volume;
        if enabled && !self.playing {
            for sound in [&self.wind, &self.day, &self.night].into_iter().flatten() {
                play_sound(
                    sound,
                    PlaySoundParams {
                        looped: true,
                        volume: 0.,
                    },
                );
            }
            self.playing = true;
        } else if !enabled && self.playing {
            for sound in [&self.wind, &self.day, &self.night, &self.step]
                .into_iter()
                .flatten()
            {
                stop_sound(sound);
            }
            self.playing = false;
        }
        let daylight = ((hours - 5.5) / 1.5).clamp(0., 1.) * ((20. - hours) / 1.5).clamp(0., 1.);
        if let Some(s) = &self.wind {
            set_sound_volume(s, volume * (0.08 + wind.clamp(0., 2.) * 0.25));
        }
        if let Some(s) = &self.day {
            set_sound_volume(s, volume * daylight * 0.3);
        }
        if let Some(s) = &self.night {
            set_sound_volume(s, volume * (1. - daylight) * 0.16);
        }
    }
    pub fn walk(&mut self, distance: f32) {
        self.walked += distance;
        if self.walked >= 0.85 {
            self.walked %= 0.85;
            if self.enabled {
                if let Some(s) = &self.step {
                    play_sound(
                        s,
                        PlaySoundParams {
                            looped: false,
                            volume: self.volume * 0.35,
                        },
                    );
                }
            }
        }
    }
}

fn wave(kind: u32) -> Vec<u8> {
    const RATE: u32 = 22050;
    let count = if kind == 3 { RATE / 5 } else { RATE * 12 };
    let mut pcm = Vec::with_capacity(count as usize * 2);
    let mut random = 0x3748ac59u32.wrapping_add(kind);
    let mut low = 0.;
    let tau = std::f32::consts::TAU;
    for i in 0..count {
        let t = i as f32 / RATE as f32;
        random ^= random << 13;
        random ^= random >> 17;
        random ^= random << 5;
        let noise = random as f32 / u32::MAX as f32 * 2. - 1.;
        low += 0.04 * (noise - low);
        // Fade each loop's boundary so the repeated buffer never clicks.
        let attack = if kind == 3 { 0.003 } else { 0.15 };
        let release = if kind == 3 { 0.03 } else { 0.15 };
        let fade = (t / attack)
            .min((count as f32 / RATE as f32 - t) / release)
            .clamp(0., 1.);
        let value = match kind {
            0 => low * (0.6 + 0.3 * (tau * t / 4.).sin() + 0.1 * (tau * t / 1.5).sin()),
            1 => {
                let phase = t % 3.;
                let chirp = phase % 0.28;
                let gate = if (0.4..1.45).contains(&phase) && chirp < 0.13 {
                    (std::f32::consts::PI * chirp / 0.13).sin().powi(2)
                } else {
                    0.
                };
                (tau * (2100. * chirp + 5200. * chirp * chirp)).sin() * gate * 0.18
            }
            2 => {
                let gate = (tau * t * 3.5).sin().max(0.).powi(5);
                (tau * 4100. * t).sin() * gate * 0.07
            }
            _ => {
                let env = (-35. * t).exp();
                (noise * 0.2 + (tau * 95. * t).sin() * 0.3) * env
            }
        } * fade;
        pcm.extend_from_slice(&((value.clamp(-1., 1.) * i16::MAX as f32) as i16).to_le_bytes());
    }
    let bytes = pcm.len() as u32;
    let mut wav = Vec::with_capacity(pcm.len() + 44);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(bytes + 36).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes());
    wav.extend_from_slice(&RATE.to_le_bytes());
    wav.extend_from_slice(&(RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&bytes.to_le_bytes());
    wav.extend(pcm);
    wav
}

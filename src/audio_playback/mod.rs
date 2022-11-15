mod kdmapi;
mod xsynth;

pub enum AudioPlayerType {
    XSynth(String, f64),
    Kdmapi,
}

pub struct SimpleTemporaryPlayer {
    player_type: AudioPlayerType,
    xsynth: Option<xsynth::XSynthPlayer>,
    kdmapi: Option<kdmapi::KDMAPIPlayer>,
}

impl SimpleTemporaryPlayer {
    pub fn new(player_type: AudioPlayerType) -> Self {
        let (xsynth, kdmapi) = match player_type {
            AudioPlayerType::XSynth(ref sfz, buf) => {
                let xsynth = xsynth::XSynthPlayer::new(sfz, buf);
                (Some(xsynth), None)
            }
            AudioPlayerType::Kdmapi => {
                let kdmapi = kdmapi::KDMAPIPlayer::new();
                (None, Some(kdmapi))
            }
        };
        Self {
            player_type,
            xsynth,
            kdmapi,
        }
    }

    pub fn get_voice_count(&self) -> u64 {
        match self.player_type {
            AudioPlayerType::XSynth(..) => {
                if let Some(xsynth) = &self.xsynth {
                    xsynth.get_voice_count()
                } else {
                    0
                }
            }
            AudioPlayerType::Kdmapi => 0,
        }
    }

    pub fn push_events(&mut self, data: impl Iterator<Item = u32>) {
        for e in data {
            self.push_event(e);
        }
    }

    pub fn push_event(&mut self, data: u32) {
        match self.player_type {
            AudioPlayerType::XSynth(..) => {
                if let Some(xsynth) = self.xsynth.as_mut() {
                    xsynth.push_event(data)
                }
            }
            AudioPlayerType::Kdmapi => {
                if let Some(kdmapi) = self.kdmapi.as_mut() {
                    kdmapi.push_event(data)
                }
            }
        }
    }

    pub fn reset(&mut self) {
        match self.player_type {
            AudioPlayerType::XSynth(..) => {
                if let Some(xsynth) = self.xsynth.as_mut() {
                    xsynth.reset()
                }
            }
            AudioPlayerType::Kdmapi => {
                if let Some(kdmapi) = self.kdmapi.as_mut() {
                    kdmapi.reset()
                }
            }
        }
    }

    pub fn set_layer_count(&mut self, layers: Option<usize>) {
        if let AudioPlayerType::XSynth(..) = self.player_type {
            if let Some(xsynth) = self.xsynth.as_mut() {
                xsynth.set_layer_count(layers)
            }
        }
    }

    pub fn set_soundfont(&mut self, path: &str) {
        if let AudioPlayerType::XSynth(..) = self.player_type {
            if let Some(xsynth) = self.xsynth.as_mut() {
                xsynth.set_soundfont(path)
            }
        }
    }
}

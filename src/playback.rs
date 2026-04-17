#[derive(Default, PartialEq, Eq, Clone, Copy, Debug)]
pub enum PlayingState {
    #[default]
    Stopped,
    Playing,
    Recording,
    OfflineRendering,
}

impl PlayingState {
    pub fn is_playing(&self) -> bool {
        match self {
            PlayingState::Stopped => false,
            PlayingState::Playing => true,
            PlayingState::Recording => true,
            PlayingState::OfflineRendering => true,
        }
    }

    pub fn play_pause(&mut self) {
        match self {
            PlayingState::Playing => *self = PlayingState::Stopped,
            PlayingState::Stopped => *self = PlayingState::Playing,
            PlayingState::Recording => *self = PlayingState::Stopped,
            PlayingState::OfflineRendering => *self = PlayingState::OfflineRendering,
        }

        // TODO: fire out thing for this
        // unsafe_globals().contiguous_playback = false;

        // accessibility::read_player_state_change();
    }

    pub fn tts_name(&self) -> String {
        match self {
            PlayingState::Stopped => "Stopped".to_string(),
            PlayingState::Playing => "Playing".to_string(),
            PlayingState::Recording => "Recording".to_string(),
            PlayingState::OfflineRendering => "Offline Rendering".to_string(),
        }
    }
}

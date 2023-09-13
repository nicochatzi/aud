pub struct AppState {
    link: rusty_link::AblLink,
    session_state: rusty_link::SessionState,
    quantum: f64,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            link: rusty_link::AblLink::new(120.),
            session_state: rusty_link::SessionState::new(),
            quantum: 4.,
        }
    }

    pub fn with_on_state() -> Self {
        let mut this = Self::new();
        this.enable_start_stop_sync(!this.is_start_stop_sync_enabled());
        this.enable(!this.is_enabled());
        this
    }

    pub fn capture_session_state(&mut self) {
        self.link.capture_app_session_state(&mut self.session_state);
    }

    pub fn commit_session_state(&mut self) {
        self.link.commit_app_session_state(&self.session_state);
    }

    pub fn on_tick(&mut self) {
        self.capture_session_state();
    }

    pub fn time(&self) -> i64 {
        self.link.clock_micros()
    }

    // pub fn phase(&self) -> f64 {
    //     self.session_state.phase_at_time(self.time(), self.quantum)
    // }

    pub fn is_start_stop_sync_enabled(&self) -> bool {
        self.link.is_start_stop_sync_enabled()
    }

    pub fn is_enabled(&self) -> bool {
        self.link.is_enabled()
    }

    pub fn is_playing(&self) -> bool {
        self.session_state.is_playing()
    }

    pub fn num_peers(&self) -> u64 {
        self.link.num_peers()
    }

    pub fn tempo(&self) -> f64 {
        self.session_state.tempo()
    }

    pub fn beats(&self) -> f64 {
        self.session_state.beat_at_time(self.time(), self.quantum)
    }

    pub fn stop(&mut self) {
        self.link.enable(false);
    }

    pub fn quantum(&self) -> f64 {
        self.quantum
    }

    pub fn enable(&mut self, should_enable: bool) {
        self.link.enable(should_enable);
    }

    pub fn enable_start_stop_sync(&mut self, should_enable: bool) {
        self.link.enable_start_stop_sync(should_enable);
    }

    pub fn set_quantum(&mut self, quantum: f64) {
        self.quantum = quantum.clamp(1., 16.);
    }

    pub fn set_session_tempo(&mut self, tempo: f64) {
        self.session_state
            .set_tempo(tempo.clamp(20.0, 999.), self.time());
    }

    pub fn toggle_session_is_playing(&mut self) {
        if self.session_state.is_playing() {
            self.session_state.set_is_playing(false, self.time() as u64);
        } else {
            self.session_state.set_is_playing_and_request_beat_at_time(
                true,
                self.time() as u64,
                0.,
                self.quantum(),
            );
        }
    }
}

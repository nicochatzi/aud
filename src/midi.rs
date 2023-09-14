use midly::{
    live::{LiveEvent, MtcQuarterFrameMessage, SystemCommon, SystemRealtime},
    MidiMessage,
};

pub struct MidiMessageString {
    pub timestamp: u64,
    pub category: String,
    pub data: String,
}

impl MidiMessageString {
    pub fn new(timestamp: u64, bytes: &[u8]) -> Option<Self> {
        let Ok(event) = LiveEvent::parse(bytes) else {
            return None;
        };

        let make = |category: &str, data: &str| Self {
            timestamp,
            category: category.to_string(),
            data: data.to_string(),
        };

        let str = match event {
            LiveEvent::Midi { channel, message } => {
                let make = |cat: &str, data: &str| make(cat, &format!("chan = {channel} | {data}"));

                match message {
                    MidiMessage::NoteOn { key, vel } => {
                        make("NoteOn", &format!("key = {key} | vel = {vel}"))
                    }
                    MidiMessage::NoteOff { key, vel } => {
                        make("NoteOff", &format!("key = {key} | vel = {vel}"))
                    }
                    MidiMessage::Aftertouch { key, vel } => {
                        make("Aftertouch", &format!("key = {key} | vel = {vel}"))
                    }
                    MidiMessage::Controller { controller, value } => {
                        make("Controller", &format!("cc = {controller} | val = {value}"))
                    }
                    MidiMessage::ProgramChange { program } => {
                        make("ProgramChange", &format!("program = {program}"))
                    }
                    MidiMessage::ChannelAftertouch { vel } => {
                        make("ChannelAftertouch", &format!("vel = {vel}"))
                    }
                    MidiMessage::PitchBend { bend } => {
                        make("PitchBend", &format!("bend = {}", bend.as_int()))
                    }
                }
            }
            LiveEvent::Common(event) => match event {
                SystemCommon::SysEx(sysex) => make("SysEx", &format!("len={}", sysex.len())),
                SystemCommon::MidiTimeCodeQuarterFrame(frame, val) => {
                    let frame = match frame {
                        MtcQuarterFrameMessage::FramesLow => "frames:lo",
                        MtcQuarterFrameMessage::FramesHigh => "frames:hi",
                        MtcQuarterFrameMessage::SecondsLow => "seconds:lo",
                        MtcQuarterFrameMessage::SecondsHigh => "seconds:",
                        MtcQuarterFrameMessage::MinutesLow => "minutes:lo",
                        MtcQuarterFrameMessage::MinutesHigh => "minutes:hi",
                        MtcQuarterFrameMessage::HoursLow => "hours:lo",
                        MtcQuarterFrameMessage::HoursHigh => "hours:hi",
                    };
                    make("TimeCode", &format!("frame = {frame} | val = {val}"))
                }
                SystemCommon::SongPosition(pos) => make("SongPos", &format!("pos = {pos}")),
                SystemCommon::SongSelect(song) => make("SongSel", &format!("song = {song}")),
                SystemCommon::TuneRequest => make("TuneReq", "_"),
                SystemCommon::Undefined(byte, _) => make("Undefined", &format!("byte = {byte}")),
            },
            LiveEvent::Realtime(event) => match event {
                SystemRealtime::TimingClock => make("Clock", "_"),
                SystemRealtime::Start => make("Start", "_"),
                SystemRealtime::Continue => make("Continue", "_"),
                SystemRealtime::Stop => make("Stop", "_"),
                SystemRealtime::ActiveSensing => make("ActiveSensing", "_"),
                SystemRealtime::Reset => make("Reset", "_"),
                SystemRealtime::Undefined(byte) => make("Undefined", &format!("{byte}")),
            },
        };

        Some(str)
    }
}

pub mod ableton_link;
pub mod audio;
pub mod audio_midi;
pub mod audio_remote;
pub mod midi;

#[cfg(test)]
mod test {
    use super::audio_midi::{AppEvent, AudioMidiController};
    use crate::midi::{MidiData, MidiReceiving};
    use std::time::Duration;

    const MIDI_DEVICES: &[&str] = &["dev0", "dev1", "dev2"];
    const MIDI_BYTES: &[u8] = &[1, 2, 3];
    const TIMEOUT: Duration = Duration::from_millis(500);

    #[derive(Default)]
    struct MockMidiHost {
        is_active: bool,
    }

    impl MidiReceiving for MockMidiHost {
        fn is_midi_stream_active(&self) -> bool {
            self.is_active
        }

        fn set_midi_stream_active(&mut self, should_activate: bool) {
            self.is_active = should_activate;
        }

        fn list_midi_devices(&self) -> anyhow::Result<Vec<String>> {
            Ok(MIDI_DEVICES.iter().map(|s| s.to_string()).collect())
        }

        fn connect_to_midi_device(&mut self, device_name: &str) -> anyhow::Result<()> {
            assert!(MIDI_DEVICES.contains(&device_name));
            Ok(())
        }

        fn produce_midi_messages(&mut self) -> Vec<MidiData> {
            vec![MidiData {
                timestamp: 1111,
                bytes: MIDI_BYTES.into(),
            }]
        }
    }

    #[test]
    fn is_off_by_default() {
        let mut app = AudioMidiController::with_midi(Box::<MockMidiHost>::default(), "");
        assert!(!app.midi().is_running());
        assert!(app.take_alert().is_none());

        app.midi_mut().set_running(true);
        assert!(app.midi().is_running());
    }

    #[test]
    fn can_load_a_script_and_receive_an_alert() {
        let mut app = AudioMidiController::with_midi(Box::<MockMidiHost>::default(), "");

        let script = crate::test::fixture("alert_on_load.lua");
        app.load_script_sync(script.clone(), TIMEOUT).unwrap();
        assert_eq!(*app.loaded_script_path().unwrap(), script);

        assert_eq!(app.process_script_events().unwrap(), AppEvent::Continue);
        assert_eq!(app.take_alert().unwrap(), "loaded");
    }

    #[test]
    fn can_call_into_scripts_through_hooks() {
        let mut app = AudioMidiController::with_midi(Box::<MockMidiHost>::default(), "");

        let script = crate::test::fixture("alert_in_hooks.lua");
        app.load_script_sync(script.clone(), TIMEOUT).unwrap();
        assert_eq!(*app.loaded_script_path().unwrap(), script);

        assert_eq!(app.process_script_events().unwrap(), AppEvent::Continue);
        assert_eq!(app.take_alert().unwrap(), "on_start");

        let devices = MIDI_DEVICES
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .join(",");

        assert_eq!(
            app.wait_for_alert(TIMEOUT).unwrap().unwrap(),
            format!("on_discover:{}", devices)
        );

        app.midi_mut().connect_to_input_by_index(0).unwrap();
        assert_eq!(app.midi().selected_port_name().unwrap(), MIDI_DEVICES[0]);

        assert_eq!(
            app.wait_for_alert(TIMEOUT).unwrap().unwrap(),
            format!("on_connect:{}", MIDI_DEVICES[0])
        );

        app.midi_mut().update();

        let bytes = MIDI_BYTES
            .iter()
            .map(|b| b.to_string())
            .collect::<Vec<_>>()
            .join(",");

        assert_eq!(
            app.wait_for_alert(TIMEOUT).unwrap().unwrap(),
            format!("on_midi:{}:{bytes}", MIDI_DEVICES[0])
        );
    }

    #[test]
    fn does_not_panic_when_an_invalid_script_crashes_the_engine() {
        let mut app = AudioMidiController::with_midi(Box::<MockMidiHost>::default(), "");

        let invalid_script = crate::test::fixture("invalid.lua");
        app.load_script_sync(invalid_script, TIMEOUT).unwrap_err();
        assert_eq!(app.process_engine_events().unwrap(), AppEvent::ScriptCrash);

        let valid_script = crate::test::fixture("alert_on_load.lua");
        app.load_script_sync(valid_script, TIMEOUT).unwrap();
        assert_eq!(app.process_engine_events().unwrap(), AppEvent::Continue);
    }
}

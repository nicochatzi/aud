mod app;
mod lua;

pub use app::*;

#[cfg(test)]
mod test {
    use super::*;
    use crate::midi::{MidiData, MidiReceiving};
    use std::time::Duration;

    const MIDI_DEV: &str = "mock-midi-device";
    const MIDI_BYTES: &[u8] = &[1, 2, 3];
    const TIMEOUT: Duration = Duration::from_secs(1);

    #[derive(Default)]
    struct EmptyMidiHost;
    impl MidiReceiving for EmptyMidiHost {
        fn is_midi_stream_active(&self) -> bool {
            true
        }

        fn set_midi_stream_active(&mut self, _: bool) {}

        fn list_midi_devices(&self) -> anyhow::Result<Vec<String>> {
            Ok(vec![MIDI_DEV.into()])
        }

        fn connect_to_midi_device(&mut self, _device_name: &str) -> anyhow::Result<()> {
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
    fn can_load_a_script_and_receive_an_alert() {
        let mut app = App::new(Box::<EmptyMidiHost>::default());
        assert!(app.running());
        assert!(app.take_alert().is_none());

        let script = crate::test::fixture("alert_on_load.lua");
        assert_eq!(app.load_script(script.clone()).unwrap(), AppEvent::Continue);
        assert_eq!(*app.loaded_script_path().unwrap(), script);

        app.wait_for_script_to_load(TIMEOUT).unwrap();
        assert_eq!(app.process_script_events().unwrap(), AppEvent::Continue);
        assert_eq!(app.take_alert().unwrap(), "loaded");
    }

    #[test]
    fn can_call_into_scripts_through_hooks() {
        let mut app = App::new(Box::<EmptyMidiHost>::default());
        assert!(app.running());
        assert!(app.take_alert().is_none());

        let script = crate::test::fixture("alert_in_hooks.lua");
        assert_eq!(app.load_script(script.clone()).unwrap(), AppEvent::Continue);
        assert_eq!(*app.loaded_script_path().unwrap(), script);

        app.wait_for_script_to_load(TIMEOUT).unwrap();
        assert_eq!(app.process_script_events().unwrap(), AppEvent::Continue);
        assert_eq!(app.take_alert().unwrap(), "on_start");

        assert_eq!(
            app.wait_for_alert(TIMEOUT).unwrap(),
            format!("on_discover:{MIDI_DEV}")
        );

        app.connect_to_midi_input(MIDI_DEV).unwrap();

        assert_eq!(
            app.wait_for_alert(TIMEOUT).unwrap(),
            format!("on_connect:{MIDI_DEV}")
        );

        app.process_midi_messages();

        let bytes = MIDI_BYTES
            .iter()
            .map(|b| b.to_string())
            .collect::<Vec<String>>()
            .join(",");

        assert_eq!(
            app.wait_for_alert(TIMEOUT).unwrap(),
            format!("on_midi:{MIDI_DEV}:{bytes}")
        );
    }
}

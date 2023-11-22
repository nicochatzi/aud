use crate::ui::{components, widgets};
use aud::{apps::audio_midi::AudioMidiController, audio::AudioDevice, files};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::prelude::*;

const USAGE: &str = r#"
         ? : display help
         a : display API
         s : display script
         d : display docs
         K : increase gain
         J : decrease gain
         H : zoom out
         L : zoom in
   <UP>, k : scroll up
 <DOWN>, j : scroll down
 <LEFT>, h : cycle panes left
<RIGHT>, l : cycle panes right
     Enter : confirm selection
  <ESC>, q : quit or hide help
     <C-c> : force quit
"#;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Popup {
    Usage,
    Api,
    Docs,
    Script,
    Alert,
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum Selector {
    Device,
    Script,
}

pub enum UiEvent<Id> {
    Continue,
    Select { id: Id, index: usize },
    LoadScript(usize),
    Exit,
}

pub struct Ui {
    popups: components::Popups<Popup>,
    selectors: components::Selectors<Selector>,
    script_dir: Option<std::path::PathBuf>,
    script_names: Vec<String>,
    alert_message: Option<String>,
    cached_script: Option<String>,
    downsample: usize,
    gain: f32,
}

impl Default for Ui {
    fn default() -> Self {
        Self {
            popups: components::Popups::new(&[
                (Popup::Usage, components::PopupKind::Text),
                (Popup::Api, components::PopupKind::Code),
                (Popup::Docs, components::PopupKind::Code),
                (Popup::Script, components::PopupKind::Code),
                (Popup::Alert, components::PopupKind::Text),
            ]),
            selectors: components::Selectors::new(&[Selector::Device, Selector::Script]),
            script_dir: None,
            script_names: vec![],
            alert_message: None,
            cached_script: None,
            downsample: 16,
            gain: 1.,
        }
    }
}

impl Ui {
    const SAMPLE_RATE: usize = 48000;

    pub fn scripts(&self) -> &[String] {
        self.script_names.as_slice()
    }

    pub fn script_dir(&self) -> Option<&std::path::PathBuf> {
        self.script_dir.as_ref()
    }

    pub fn clear_script_cache(&mut self) {
        self.cached_script = None;
    }

    pub fn update_script_dir(&mut self, dir: impl AsRef<std::path::Path>) -> anyhow::Result<()> {
        let dir = dir.as_ref();
        self.script_names = files::list_with_extension(dir, "lua")?;
        self.script_dir = Some(dir.into());
        if let Some(sel) = self.selectors.get_mut(Selector::Script) {
            *sel = components::Selector::with_len(self.script_names.len());
        }
        Ok(())
    }

    pub fn update_device_names(&mut self, names: &[AudioDevice]) {
        if let Some(devices) = self.selectors.get_mut(Selector::Device) {
            *devices = components::Selector::with_len(names.len());
        };
    }

    fn adjust_gain(&mut self, amount: f32) {
        self.gain = (self.gain + amount).clamp(0., 16.);
    }

    fn adjust_downsample(&mut self, amount: isize) {
        self.downsample = (self.downsample as isize + amount).clamp(8, 4096) as usize;
    }

    pub fn on_keypress(&mut self, key: KeyEvent) -> UiEvent<Selector> {
        match key.code {
            KeyCode::Char('?') => self.popups.toggle_visible(Popup::Usage),
            KeyCode::Char('a') => self.popups.toggle_visible(Popup::Api),
            KeyCode::Char('s') => self.popups.toggle_visible(Popup::Script),
            KeyCode::Char('d') => self.popups.toggle_visible(Popup::Docs),
            KeyCode::Char('q') | KeyCode::Esc => {
                if !self.popups.any_visible() {
                    return UiEvent::Exit;
                }
                self.popups.hide()
            }
            KeyCode::Char('K') => self.adjust_gain(0.1),
            KeyCode::Char('J') => self.adjust_gain(-0.1),
            KeyCode::Char('H') => self.adjust_downsample(-8),
            KeyCode::Char('L') => self.adjust_downsample(8),
            KeyCode::Up | KeyCode::Char('k') => self.selectors.previous_item(),
            KeyCode::Down | KeyCode::Char('j') => self.selectors.next_item(),
            KeyCode::Left | KeyCode::Char('h') => self.selectors.previous_selector(),
            KeyCode::Right | KeyCode::Char('l') => self.selectors.next_selector(),
            KeyCode::Enter => {
                if let Some(selection) = self.selectors.select() {
                    return match selection.selector {
                        Selector::Script => {
                            self.cached_script = None;
                            UiEvent::LoadScript(selection.index)
                        }
                        Selector::Device => UiEvent::Select {
                            id: selection.selector,
                            index: selection.index,
                        },
                    };
                }
            }
            _ => {}
        }

        UiEvent::Continue
    }

    pub fn show_alert_message(&mut self, alert_message: &str) {
        self.popups.show(Popup::Alert);
        self.alert_message = Some(alert_message.into());
    }

    pub fn render(&mut self, f: &mut Frame, app: &AudioMidiController) {
        let sections = Layout::default()
            .direction(Direction::Horizontal)
            .margin(1)
            .constraints([Constraint::Min(32), Constraint::Percentage(90)].as_ref())
            .split(f.size());

        let left_sections = Layout::default()
            .direction(Direction::Vertical)
            .margin(0)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(sections[0]);

        let has_script_dir = self.script_dir.as_ref().is_some_and(|dir| dir.is_dir());
        let device_selector_section = if has_script_dir {
            left_sections[0]
        } else {
            sections[0]
        };

        self.selectors.render(
            f,
            device_selector_section,
            Selector::Device,
            crate::title!("devices"),
            &app.audio()
                .devices()
                .iter()
                .map(|d| d.name.clone())
                .collect::<Vec<_>>(),
        );

        if has_script_dir {
            self.selectors.render(
                f,
                left_sections[1],
                Selector::Script,
                &crate::title!("{}", self.script_dir.as_ref().unwrap().to_string_lossy()),
                self.script_names.as_slice(),
            );
        }

        let selected_device_name = self
            .selectors
            .get(Selector::Device)
            .and_then(|s| {
                s.selected()
                    .and_then(|index| app.audio().devices().get(index))
            })
            .map(|device| device.name.clone())
            .unwrap_or_default();

        let scope_tile = format!(
            "{}───{}─{}",
            crate::title!("{}", selected_device_name),
            crate::title!("zoom : {}", self.downsample),
            crate::title!("gain : {:.2}", self.gain),
        );

        widgets::scope::render(
            f,
            sections[1],
            &scope_tile,
            app.audio().buffer(),
            self.downsample,
            self.gain,
        );

        self.popups.render(
            f,
            Popup::Api,
            crate::title!("api"),
            aud::lua::imported::auscope::API,
        );

        self.popups.render(
            f,
            Popup::Docs,
            crate::title!("docs"),
            aud::lua::imported::auscope::DOCS,
        );

        self.popups
            .render(f, Popup::Usage, crate::title!("usage"), USAGE);

        self.popups.render(
            f,
            Popup::Alert,
            crate::title!("alert!"),
            self.alert_message.as_ref().unwrap_or(&"".to_owned()),
        );

        if !self.popups.is_visible(Popup::Script) {
            self.popups
                .render(f, Popup::Script, crate::title!(""), "No script loaded");
        }

        let selected_script_name = match app.selected_script() {
            Some(name) => crate::title!("script : {}", name),
            None => "".to_owned(),
        };

        if self.cached_script.is_none() {
            self.cached_script = Some(
                app.loaded_script_path()
                    .and_then(|path| std::fs::read_to_string(path).ok())
                    .unwrap_or_else(|| "No script loaded".to_owned()),
            );
        }

        self.popups.render(
            f,
            Popup::Script,
            &selected_script_name,
            self.cached_script.as_ref().unwrap(),
        );
    }

    pub fn remove_offscreen_samples(
        &mut self,
        app: &mut AudioMidiController,
        screen_width: usize,
        fps: f32,
    ) {
        let audio = app.audio_mut().buffer_mut();
        let num_renderable_samples = screen_width * self.downsample;
        let num_samples_to_purge =
            ((Self::SAMPLE_RATE as f32 / fps) * audio.num_channels as f32) as usize;

        if audio.data.len() > num_renderable_samples {
            let num_samples_to_purge =
                num_samples_to_purge.max(audio.data.len() - num_renderable_samples);

            let _ = audio.data.drain(0..num_samples_to_purge);
        }
    }
}

use relm4::prelude::*;
use relm4::component::*;

use adw::prelude::*;

use anime_launcher_sdk::config;

use std::path::PathBuf;
use anime_launcher_sdk::integrations::steam;
use std::env;

use crate::*;
use crate::i18n::*;
use super::main::*;

pub struct DefaultPathsDeckApp {
    show_additional: bool,

    launcher: PathBuf,
    runners: PathBuf,
    dxvks: PathBuf,
    prefix: PathBuf,
    game: PathBuf,
    fps_unlocker: PathBuf,
    components: PathBuf,
    patch: PathBuf,
    temp: PathBuf
}

#[derive(Debug, Clone)]
pub enum Folders {
    Launcher,
    Runners,
    DXVK,
    Prefix,
    Game,
    FpsUnlocker,
    Components,
    Patch,
    Temp
}

#[derive(Debug, Clone)]
pub enum DefaultPathsDeckAppMsg {
    ToggleShowAdditional,
    ChoosePathInternal(),
    ChoosePathExternal(),
    ChoosePath(Folders),
    Continue,
    Exit
}

#[relm4::component(async, pub)]
impl SimpleAsyncComponent for DefaultPathsDeckApp {
    type Init = ();
    type Input = DefaultPathsDeckAppMsg;
    type Output = FirstRunAppMsg;

    view! {
        adw::PreferencesPage {
            set_hexpand: true,

            add = &adw::PreferencesGroup {
                set_valign: gtk::Align::Center,
                set_vexpand: true,

                gtk::Label {
                    set_label: &tr("choose-default-paths-deck"),
                    add_css_class: "title-1"
                }
            },

            add = &adw::PreferencesGroup {
                set_valign: gtk::Align::Center,
                set_vexpand: true,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_halign: gtk::Align::Center,

                    set_spacing: 32,

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::ToggleButton {
                            add_css_class: "card",

                            set_width_request: 180,
                            set_height_request: 120,

                            #[watch]
                            set_active: model.style == LauncherStyle::Modern,

                            gtk::Image {
                                // TODO: replace with microsd icon
                                set_from_resource: Some("/org/app/images/modern.svg")
                            },

                            connect_activated => DefaultPathsDeckAppMsg::ChoosePathExternal(Folders::Game)
                        },

                        gtk::Label {
                            set_text: &tr("choose-default-paths-deck-external"),

                            set_margin_top: 16
                        }
                    },

                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,

                        gtk::ToggleButton {
                            add_css_class: "card",

                            set_width_request: 180,
                            set_height_request: 120,

                            #[watch]
                            set_active: model.style == LauncherStyle::Classic,

                            gtk::Image {
                                // todo: replace with nvme icon
                                set_from_resource: Some("/org/app/images/classic.svg")
                            },

                            connect_activated => DefaultPathsDeckAppMsg::ChoosePathInternal(Folders::Game)
                        },

                        gtk::Label {
                            set_text: &tr("choose-default-paths-deck-internal"),

                            set_margin_top: 16
                        }
                    }
                }
            },

            add = &adw::PreferencesGroup {
                set_valign: gtk::Align::Center,
                set_vexpand: true,
    
                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_halign: gtk::Align::Center,
                    set_spacing: 8,
    
                    gtk::Button {
                        set_label: &tr("continue"),
                        set_css_classes: &["suggested-action", "pill"],

                        connect_clicked => DefaultPathsDeckAppMsg::Continue
                    },

                    gtk::Button {
                        set_label: &tr("exit"),
                        add_css_class: "pill",

                        connect_clicked => DefaultPathsDeckAppMsg::Exit
                    }
                }
            }
        }
    }

    async fn init(
        _init: Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = Self {
            show_additional: false,

            launcher: LAUNCHER_FOLDER.to_path_buf(),
            runners: CONFIG.game.wine.builds.clone(),
            dxvks: CONFIG.game.dxvk.builds.clone(),
            prefix: CONFIG.game.wine.prefix.clone(),
            game: CONFIG.game.path.clone(),
            fps_unlocker: CONFIG.game.enhancements.fps_unlocker.path.clone(),
            components: CONFIG.components.path.clone(),
            patch: CONFIG.patch.path.clone(),

            #[allow(clippy::or_fun_call)]
            temp: CONFIG.launcher.temp.clone().unwrap_or(std::env::temp_dir())
        };

        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            DefaultPathsDeckAppMsg::ToggleShowAdditional => self.show_additional = !self.show_additional,
            DefaultPathsDeckAppMsg::ChoosePathInternal() => {
                // hack lol
                self.game = {
                    match env::var("STEAM_COMPAT_DATA_PATH") {
                        Ok(compat_path) => {
                            compat_path.path().to_path_buf().join("pfx/drive_c/Program Files/Genshin Impact")
                        }
                    },
                    Err(_) => {}
                }
            },
            DefaultPathsDeckAppMsg::ChoosePathExternal() => {
                // hack lol
                self.game         = result
            },

            DefaultPathsDeckAppMsg::ChoosePath(folder) => {
                let result = rfd::AsyncFileDialog::new()
                    .set_directory(&self.launcher)
                    .pick_folder().await;

                if let Some(result) = result {
                    let result = result.path().to_path_buf();

                    match folder {
                        Folders::Launcher => {
                            self.runners      = result.join("runners");
                            self.dxvks        = result.join("dxvks");
                            self.prefix       = result.join("prefix");
                            self.game         = result.join("Genshin Impact"); // TODO: change it based on GameEdition
                            self.fps_unlocker = result.join("fps-unlocker");
                            self.components   = result.join("components");
                            self.patch        = result.join("patch");
                            self.temp         = result.clone();

                            self.launcher = result;
                        }

                        Folders::Runners     => self.runners      = result,
                        Folders::DXVK        => self.dxvks        = result,
                        Folders::Prefix      => self.prefix       = result,
                        Folders::Game        => self.game         = result,
                        Folders::FpsUnlocker => self.fps_unlocker = result,
                        Folders::Components  => self.components   = result,
                        Folders::Patch       => self.patch        = result,
                        Folders::Temp        => self.temp         = result
                    }
                }
            }

            #[allow(unused_must_use)]
            DefaultPathsDeckAppMsg::Continue => {
                match self.update_config() {
                    Ok(_) => sender.output(Self::Output::ScrollToSelectVoiceovers),
    
                    Err(err) => sender.output(Self::Output::Toast {
                        title: tr("config-update-error"),
                        description: Some(err.to_string())
                    })
                };
            }

            DefaultPathsDeckAppMsg::Exit => relm4::main_application().quit()
        }
    }
}

impl DefaultPathsDeckApp {
    pub fn update_config(&self) -> anyhow::Result<()> {
        let mut config = config::get()?;

        config.game.wine.builds = self.runners.clone();
        config.game.dxvk.builds = self.dxvks.clone();
        config.game.wine.prefix = self.prefix.clone();
        config.game.path        = self.game.clone();
        config.components.path  = self.components.clone();
        config.patch.path       = self.patch.clone();
        config.launcher.temp    = Some(self.temp.clone());

        config.game.enhancements.fps_unlocker.path = self.fps_unlocker.clone();

        config::update_raw(config)
    }
}

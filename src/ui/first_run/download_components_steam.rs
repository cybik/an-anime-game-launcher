use relm4::prelude::*;
use relm4::component::*;

use adw::prelude::*;

use anime_launcher_sdk::components::*;
use anime_launcher_sdk::components::wine::WincompatlibWine;
use anime_launcher_sdk::anime_game_core::installer::prelude::*;
use anime_launcher_sdk::config;
use anime_launcher_sdk::wincompatlib::prelude::*;

use std::path::PathBuf;

use super::main::FirstRunAppMsg;
use crate::ui::components::*;
use crate::i18n::*;
use crate::*;
use anime_launcher_sdk::integrations::steam;

pub struct DownloadComponentsSteamApp {
    progress_bar: AsyncController<ProgressBar>,

    wine_combo: adw::ComboRow,

    wine_versions: Vec<wine::Version>,

    selected_wine: Option<wine::Version>,

    /// `None` - default,
    /// `Some(false)` - processing,
    /// `Some(true)` - done
    downloading_wine: Option<bool>,
    downloading_wine_version: String,

    downloading: bool
}

#[derive(Debug, Clone)]
pub enum DownloadComponentsSteamAppMsg {
    UpdateVersionsLists,
    ConfirmChoice,
    Continue,
    Exit
}

#[relm4::component(async, pub)]
impl SimpleAsyncComponent for DownloadComponentsSteamApp {
    type Init = ();
    type Input = DownloadComponentsSteamAppMsg;
    type Output = FirstRunAppMsg;

    view! {
        adw::PreferencesPage {
            set_hexpand: true,

            add = &adw::PreferencesGroup {
                set_valign: gtk::Align::Center,
                set_vexpand: true,

                gtk::Label {
                    set_label: &tr("download-components"),
                    add_css_class: "title-1"
                }
            },

            add = &adw::PreferencesGroup {
                set_valign: gtk::Align::Center,
                set_vexpand: true,

                #[watch]
                set_visible: !model.downloading,

                #[local_ref]
                wine_combo -> adw::ComboRow {
                    set_title: &tr("wine-version"),

                    #[watch]
                    set_model: Some(&gtk::StringList::new(model.wine_versions.iter()
                        .map(|version| version.title.as_ref())
                        .collect::<Vec<&str>>()
                        .as_slice()))
                },
            },

            add = &adw::PreferencesGroup {
                set_valign: gtk::Align::Center,
                set_vexpand: true,

                #[watch]
                set_visible: !model.downloading,

                gtk::Box {
                    set_orientation: gtk::Orientation::Horizontal,
                    set_halign: gtk::Align::Center,
                    set_spacing: 8,

                    gtk::Button {
                        set_label: &tr("confirm-choice"),
                        set_css_classes: &["suggested-action", "pill"],

                        connect_clicked => DownloadComponentsSteamAppMsg::ConfirmChoice
                    },

                    gtk::Button {
                        set_label: &tr("exit"),
                        add_css_class: "pill",

                        connect_clicked => DownloadComponentsSteamAppMsg::Exit
                    }
                }
            },

            add = &adw::PreferencesGroup {
                set_valign: gtk::Align::Center,
                set_vexpand: true,

                #[watch]
                set_visible: model.downloading,

                adw::ActionRow {
                    set_title: &tr("download-wine"),

                    #[watch]
                    set_subtitle: &model.downloading_wine_version,

                    #[watch]
                    set_icon_name: match model.downloading_wine {
                        Some(true) => Some("emblem-ok-symbolic"),
                        Some(false) => None, // Some("process-working"),
                        None => None
                    }
                }
            }
        }
    }

    async fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let model = Self {
            progress_bar: ProgressBar::builder()
                .launch(ProgressBarInit {
                    caption: None,
                    display_progress: true,
                    display_fraction: true,
                    visible: true
                })
                .detach(),

            wine_combo: adw::ComboRow::new(),

            wine_versions: vec![],

            selected_wine: None,

            downloading_wine: None,
            downloading_wine_version: String::new(),

            downloading: false
        };

        model.progress_bar.widget().set_width_request(360);

        let wine_combo = &model.wine_combo;

        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            DownloadComponentsSteamAppMsg::UpdateVersionsLists => {
                let config = Config::get().unwrap_or_else(|_| CONFIG.clone());

                // 4 latest versions of 4 first available wine group
                self.wine_versions = wine::get_groups(&config.components.path).unwrap()
                    .into_iter()
                    .take(4)
                    .flat_map(|group| group.versions.into_iter().take(4))
                    .collect();

            }

            #[allow(unused_must_use)]
            DownloadComponentsSteamAppMsg::ConfirmChoice => {
                let mut config = Config::get().unwrap_or_else(|_| CONFIG.clone());

                self.selected_wine = Some(self.wine_versions[self.wine_combo.selected() as usize].clone());

                let wine = self.selected_wine.clone().unwrap();

                config.game.wine.selected = Some(wine.name.clone());

                if let Err(err) = Config::update_raw(config) {
                    tracing::error!("Failed to update config: {err}");

                    sender.output(Self::Output::Toast {
                        title: tr("config-update-error"),
                        description: Some(err.to_string())
                    });
                }

                sender.input(DownloadComponentsSteamAppMsg::Continue);
                // lol
            }

            #[allow(unused_must_use)]
            DownloadComponentsSteamAppMsg::Continue => {
                sender.output(Self::Output::ScrollToFinish);
            }

            DownloadComponentsSteamAppMsg::Exit => relm4::main_application().quit()
        }
    }
}

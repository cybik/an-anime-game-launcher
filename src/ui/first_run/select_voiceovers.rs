use relm4::prelude::*;
use relm4::component::*;

use adw::prelude::*;

use anime_launcher_sdk::config::ConfigExt;
use anime_launcher_sdk::genshin::config::Config;

use crate::i18n::*;

use super::main::*;

use anime_launcher_sdk::integrations::steam;

pub struct SelectVoiceoversApp {
    english: gtk::Switch,
    japanese: gtk::Switch,
    korean: gtk::Switch,
    chinese: gtk::Switch
}

#[derive(Debug, Clone)]
pub enum SelectVoiceoversAppMsg {
    Continue,
    Exit
}

#[relm4::component(async, pub)]
impl SimpleAsyncComponent for SelectVoiceoversApp {
    type Init = ();
    type Input = SelectVoiceoversAppMsg;
    type Output = FirstRunAppMsg;

    view! {
        adw::PreferencesPage {
            set_hexpand: true,

            add = &adw::PreferencesGroup {
                set_valign: gtk::Align::Center,
                set_vexpand: true,

                gtk::Label {
                    set_label: &tr("select-voice-packages"),
                    add_css_class: "title-1"
                }
            },

            add = &adw::PreferencesGroup {
                set_valign: gtk::Align::Center,
                set_vexpand: true,

                adw::ActionRow {
                    set_title: &tr("english"),

                    #[local_ref]
                    add_suffix = english -> gtk::Switch {
                        set_valign: gtk::Align::Center,
                        set_state: true
                    }
                },

                adw::ActionRow {
                    set_title: &tr("japanese"),

                    #[local_ref]
                    add_suffix = japanese -> gtk::Switch {
                        set_valign: gtk::Align::Center
                    }
                },

                adw::ActionRow {
                    set_title: &tr("korean"),

                    #[local_ref]
                    add_suffix = korean -> gtk::Switch {
                        set_valign: gtk::Align::Center
                    }
                },

                adw::ActionRow {
                    set_title: &tr("chinese"),

                    #[local_ref]
                    add_suffix = chinese -> gtk::Switch {
                        set_valign: gtk::Align::Center
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

                        connect_clicked => SelectVoiceoversAppMsg::Continue
                    },

                    gtk::Button {
                        set_label: &tr("exit"),
                        add_css_class: "pill",

                        connect_clicked => SelectVoiceoversAppMsg::Exit
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
            english: gtk::Switch::new(),
            japanese: gtk::Switch::new(),
            korean: gtk::Switch::new(),
            chinese: gtk::Switch::new()
        };

        let english  = &model.english;
        let japanese = &model.japanese;
        let korean   = &model.korean;
        let chinese  = &model.chinese;

        let widgets = view_output!();

        AsyncComponentParts { model, widgets }
    }

    async fn update(&mut self, msg: Self::Input, sender: AsyncComponentSender<Self>) {
        match msg {
            #[allow(unused_must_use)]
            SelectVoiceoversAppMsg::Continue => {
                match self.update_config() {
                    Ok(_) => {
                        match steam::launched_from() {
                            steam::LaunchedFrom::Steam => sender.output(Self::Output::ScrollToDownloadSteamComponents),
                            steam::LaunchedFrom::Independent => sender.output(Self::Output::ScrollToDownloadComponents)
                        }
                    },
    
                    Err(err) => sender.output(Self::Output::Toast {
                        title: tr("config-update-error"),
                        description: Some(err.to_string())
                    })
                };
            }

            SelectVoiceoversAppMsg::Exit => relm4::main_application().quit()
        }
    }
}

impl SelectVoiceoversApp {
    pub fn update_config(&self) -> anyhow::Result<()> {
        let mut config = Config::get()?;

        config.game.voices = Vec::new();

        if self.english.state() {
            config.game.voices.push(String::from("en-us"));
        }

        if self.japanese.state() {
            config.game.voices.push(String::from("ja-jp"));
        }

        if self.korean.state() {
            config.game.voices.push(String::from("ko-kr"));
        }

        if self.chinese.state() {
            config.game.voices.push(String::from("zh-cn"));
        }

        Config::update_raw(config)
    }
}

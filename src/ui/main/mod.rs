use relm4::{
    prelude::*,
    component::*,
    actions::*,
    MessageBroker
};

use gtk::prelude::*;
use adw::prelude::*;

use gtk::glib::clone;

mod repair_game;
mod apply_patch;
mod download_wine;
mod create_prefix;
mod download_diff;
mod migrate_folder;
mod launch;

use anime_launcher_sdk::components::loader::ComponentsLoader;

use anime_launcher_sdk::config::ConfigExt;
use anime_launcher_sdk::genshin::config::Config;

use anime_launcher_sdk::genshin::config::schema::launcher::LauncherStyle;

use anime_launcher_sdk::genshin::states::*;
use anime_launcher_sdk::genshin::consts::*;

use anime_launcher_sdk::integrations::steam;

use crate::*;
use crate::i18n::*;
use crate::ui::components::*;

use super::preferences::main::*;
use super::about::*;

relm4::new_action_group!(WindowActionGroup, "win");

relm4::new_stateless_action!(LauncherFolder, WindowActionGroup, "launcher_folder");
relm4::new_stateless_action!(GameFolder, WindowActionGroup, "game_folder");
relm4::new_stateless_action!(ConfigFile, WindowActionGroup, "config_file");
relm4::new_stateless_action!(DebugFile, WindowActionGroup, "debug_file");
relm4::new_stateless_action!(WishUrl, WindowActionGroup, "wish_url");

relm4::new_stateless_action!(About, WindowActionGroup, "about");

pub static mut MAIN_WINDOW: Option<adw::ApplicationWindow> = None;
pub static mut PREFERENCES_WINDOW: Option<AsyncController<PreferencesApp>> = None;
pub static mut ABOUT_DIALOG: Option<Controller<AboutDialog>> = None;

pub struct App {
    progress_bar: AsyncController<ProgressBar>,

    toast_overlay: adw::ToastOverlay,

    loading: Option<Option<String>>,
    style: LauncherStyle,
    state: Option<LauncherState>,

    downloading: bool,
    disabled_buttons: bool
}

#[derive(Debug)]
pub enum AppMsg {
    UpdateLauncherState {
        /// Perform action when game or voice downloading is required
        /// Needed for chained executions (e.g. update one voice after another)
        perform_on_download_needed: bool,

        /// Automatically start patch applying if possible and needed
        apply_patch_if_needed: bool,

        /// Show status gathering progress page
        show_status_page: bool
    },

    /// Supposed to be called automatically on app's run when the latest game version
    /// was retrieved from the API
    SetGameDiff(Option<VersionDiff>),

    /// Supposed to be called automatically on app's run when the latest UnityPlayer patch version
    /// was retrieved from remote repos
    SetUnityPlayerPatch(Option<UnityPlayerPatch>),

    /// Supposed to be called automatically on app's run when the latest xlua patch version
    /// was retrieved from remote repos
    SetXluaPatch(Option<XluaPatch>),

    /// Supposed to be called automatically on app's run when the launcher state was chosen
    SetLauncherState(Option<LauncherState>),

    SetLauncherStyle(LauncherStyle),
    SetLoadingStatus(Option<Option<String>>),

    SetDownloading(bool),
    DisableButtons(bool),

    OpenPreferences,
    RepairGame,

    PredownloadUpdate,
    PerformAction,

    HideWindow,
    ShowWindow,

    Toast {
        title: String,
        description: Option<String>
    }
}

#[relm4::component(pub)]
impl SimpleComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();

    menu! {
        main_menu: {
            section! {
                &tr("launcher-folder") => LauncherFolder,
                &tr("game-folder") => GameFolder,
                &tr("config-file") => ConfigFile,
                &tr("debug-file") => DebugFile,
            },

            section! {
                &tr("wish-url") => WishUrl
            },

            section! {
                &tr("about") => About
            }
        }
    }

    view! {
        main_window = adw::ApplicationWindow {
            set_icon_name: Some(APP_ID),
            set_resizable: steam::is_steam_deck(),

            #[watch]
            set_default_size: (
                steam::default_window_size_width(
                match model.style {
                    LauncherStyle::Modern => 900,
                    LauncherStyle::Classic => 1094 // (w = 1280 / 730 * h, where 1280x730 is default background picture resolution)
                }),
                steam::default_window_size_height(
                match model.style {
                    LauncherStyle::Modern => 600,
                    LauncherStyle::Classic => 624
                }
                )
            ),

            #[watch]
            set_css_classes: &{
                let mut classes = vec!["background", "csd"];

                if APP_DEBUG {
                    classes.push("devel");
                }

                match model.style {
                    LauncherStyle::Modern => (),
                    LauncherStyle::Classic => {
                        if model.loading.is_none() {
                            classes.push("classic-style");
                        }
                    }
                }

                classes
            },

            #[local_ref]
            toast_overlay -> adw::ToastOverlay {
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,

                    adw::HeaderBar {
                        #[watch]
                        set_css_classes: match model.style {
                            LauncherStyle::Modern => &[""],
                            LauncherStyle::Classic => &["flat"]
                        },

                        #[wrap(Some)]
                        set_title_widget = &adw::WindowTitle {
                            #[watch]
                            set_title: match model.style {
                                LauncherStyle::Modern => "An Anime Game Launcher",
                                LauncherStyle::Classic => ""
                            }
                        },

                        pack_end = &gtk::MenuButton {
                            set_icon_name: "open-menu-symbolic",
                            set_menu_model: Some(&main_menu)
                        }
                    },

                    adw::StatusPage {
                        set_title: &tr("loading-data"),
                        set_icon_name: Some(APP_ID),
                        set_vexpand: true,

                        #[watch]
                        set_description: match &model.loading {
                            Some(Some(desc)) => Some(desc),
                            Some(None) | None => None
                        },

                        #[watch]
                        set_visible: model.loading.is_some()
                    },

                    adw::PreferencesPage {
                        #[watch]
                        set_visible: model.loading.is_none(),

                        add = &adw::PreferencesGroup {
                            set_margin_top: 48,

                            #[watch]
                            set_visible: model.style == LauncherStyle::Modern,

                            gtk::Picture {
                                set_resource: Some("/org/app/images/icon.png"),
                                set_vexpand: true,
                                //set_content_fit: gtk::ContentFit::ScaleDown
                            },

                            gtk::Label {
                                set_label: "An Anime Game Launcher",
                                set_margin_top: 32,
                                add_css_class: "title-1"
                            }
                        },

                        add = &adw::PreferencesGroup {
                            #[watch]
                            set_valign: match model.style {
                                LauncherStyle::Modern => gtk::Align::Center,
                                LauncherStyle::Classic => gtk::Align::End
                            },

                            #[watch]
                            set_width_request: match model.style {
                                LauncherStyle::Modern => -1,
                                LauncherStyle::Classic => 800
                            },

                            #[watch]
                            set_visible: model.downloading,

                            set_vexpand: true,
                            set_margin_top: 48,
                            set_margin_bottom: 48,

                            add = model.progress_bar.widget(),
                        },

                        add = &adw::PreferencesGroup {
                            #[watch]
                            set_valign: match model.style {
                                LauncherStyle::Modern => gtk::Align::Center,
                                LauncherStyle::Classic => gtk::Align::End
                            },

                            #[watch]
                            set_width_request: match model.style {
                                LauncherStyle::Modern => -1,
                                LauncherStyle::Classic => 800
                            },

                            #[watch]
                            set_visible: !model.downloading,

                            #[watch]
                            set_margin_bottom: match model.style {
                                LauncherStyle::Modern => 48,
                                LauncherStyle::Classic => 0
                            },

                            set_vexpand: true,

                            gtk::Box {
                                #[watch]
                                set_halign: match model.style {
                                    LauncherStyle::Modern => gtk::Align::Center,
                                    LauncherStyle::Classic => gtk::Align::End
                                },

                                #[watch]
                                set_height_request: match model.style {
                                    LauncherStyle::Modern => -1,
                                    LauncherStyle::Classic => 40
                                },

                                set_margin_top: 64,
                                set_spacing: 8,

                                adw::Bin {
                                    set_css_classes: &["background", "round-bin"],

                                    gtk::Button {
                                        set_width_request: 44,

                                        #[watch]
                                        set_tooltip_text: Some(&tr_args("predownload-update", [
                                            ("version", match model.state.as_ref() {
                                                Some(LauncherState::PredownloadAvailable { game, .. }) => game.latest().to_string(),
                                                _ => String::from("?")
                                            }.into()),

                                            ("size", match model.state.as_ref() {
                                                Some(LauncherState::PredownloadAvailable { game, voices }) => {
                                                    let mut size = game.downloaded_size().unwrap_or(0);

                                                    for voice in voices {
                                                        size += voice.downloaded_size().unwrap_or(0);
                                                    }

                                                    prettify_bytes(size)
                                                }

                                                _ => String::from("?")
                                            }.into())
                                        ])),

                                        #[watch]
                                        set_visible: matches!(model.state.as_ref(), Some(LauncherState::PredownloadAvailable { .. })),

                                        #[watch]
                                        set_sensitive: match model.state.as_ref() {
                                            Some(LauncherState::PredownloadAvailable { game, voices }) => {
                                                let config = Config::get().unwrap();
                                                let temp = config.launcher.temp.unwrap_or_else(std::env::temp_dir);

                                                let downloaded = temp.join(game.file_name().unwrap()).exists() &&
                                                    voices.iter().all(|voice| temp.join(voice.file_name().unwrap()).exists());

                                                !downloaded
                                            }

                                            _ => false
                                        },

                                        #[watch]
                                        set_css_classes: match model.state.as_ref() {
                                            Some(LauncherState::PredownloadAvailable { game, voices }) => {
                                                let config = Config::get().unwrap();
                                                let temp = config.launcher.temp.unwrap_or_else(std::env::temp_dir);

                                                let downloaded = temp.join(game.file_name().unwrap()).exists() &&
                                                    voices.iter().all(|voice| temp.join(voice.file_name().unwrap()).exists());

                                                if downloaded {
                                                    &["success", "circular"]
                                                } else {
                                                    &["warning", "circular"]
                                                }
                                            }

                                            _ => &["warning", "circular"]
                                        },

                                        set_icon_name: "document-save-symbolic",
                                        set_hexpand: false,

                                        connect_clicked => AppMsg::PredownloadUpdate
                                    }
                                },

                                adw::Bin {
                                    set_css_classes: &["background", "round-bin"],

                                    gtk::Button {
                                        adw::ButtonContent {
                                            #[watch]
                                            set_icon_name: match &model.state {
                                                Some(LauncherState::Launch) |
                                                Some(LauncherState::PredownloadAvailable { .. }) => "media-playback-start-symbolic",

                                                Some(LauncherState::FolderMigrationRequired { .. }) |
                                                Some(LauncherState::WineNotInstalled) |
                                                Some(LauncherState::PrefixNotExists) => "document-save-symbolic",

                                                Some(LauncherState::GameUpdateAvailable(_)) |
                                                Some(LauncherState::GameNotInstalled(_)) |
                                                Some(LauncherState::VoiceUpdateAvailable(_)) |
                                                Some(LauncherState::VoiceNotInstalled(_)) => "document-save-symbolic",

                                                Some(LauncherState::UnityPlayerPatchAvailable(UnityPlayerPatch { status, .. })) |
                                                Some(LauncherState::XluaPatchAvailable(XluaPatch { status, .. })) => match status {
                                                    PatchStatus::NotAvailable |
                                                    PatchStatus::Outdated { .. } |
                                                    PatchStatus::Preparation { .. } => "window-close-symbolic",

                                                    PatchStatus::Testing { .. } |
                                                    PatchStatus::Available { .. } => "document-save-symbolic"
                                                }

                                                Some(LauncherState::VoiceOutdated(_)) |
                                                Some(LauncherState::GameOutdated(_)) |
                                                None => "window-close-symbolic"
                                            },

                                            #[watch]
                                            set_label: &match &model.state {
                                                Some(LauncherState::Launch) |
                                                Some(LauncherState::PredownloadAvailable { .. }) => tr("launch"),

                                                Some(LauncherState::FolderMigrationRequired { .. }) => tr("migrate-folders"),

                                                Some(LauncherState::UnityPlayerPatchAvailable(_)) |
                                                Some(LauncherState::XluaPatchAvailable(_)) => tr("apply-patch"),

                                                Some(LauncherState::WineNotInstalled) => tr("download-wine"),
                                                Some(LauncherState::PrefixNotExists)  => tr("create-prefix"),

                                                Some(LauncherState::GameUpdateAvailable(diff)) |
                                                Some(LauncherState::GameOutdated(diff)) |
                                                Some(LauncherState::VoiceUpdateAvailable(diff)) |
                                                Some(LauncherState::VoiceOutdated(diff)) => {
                                                    match (Config::get(), diff.file_name()) {
                                                        (Ok(config), Some(filename)) => {
                                                            let temp = config.launcher.temp.unwrap_or_else(std::env::temp_dir);

                                                            if temp.join(filename).exists() {
                                                                tr("resume")
                                                            }

                                                            else {
                                                                tr("update")
                                                            }
                                                        }

                                                        _ => tr("update")
                                                    }
                                                },

                                                Some(LauncherState::GameNotInstalled(_)) |
                                                Some(LauncherState::VoiceNotInstalled(_)) => tr("download"),

                                                None => String::from("...")
                                            }
                                        },

                                        #[watch]
                                        set_sensitive: !model.disabled_buttons && match &model.state {
                                            Some(LauncherState::GameOutdated { .. }) |
                                            Some(LauncherState::VoiceOutdated(_)) => false,

                                            Some(LauncherState::UnityPlayerPatchAvailable(UnityPlayerPatch { status, .. })) |
                                            Some(LauncherState::XluaPatchAvailable(XluaPatch { status, .. })) => match status {
                                                PatchStatus::NotAvailable |
                                                PatchStatus::Outdated { .. } |
                                                PatchStatus::Preparation { .. } => false,

                                                PatchStatus::Testing { .. } |
                                                PatchStatus::Available { .. } => true
                                            },

                                            Some(_) => true,

                                            None => false
                                        },

                                        #[watch]
                                        set_css_classes: match &model.state {
                                            Some(LauncherState::GameOutdated { .. }) |
                                            Some(LauncherState::VoiceOutdated(_)) => &["warning", "pill"],

                                            Some(LauncherState::UnityPlayerPatchAvailable(UnityPlayerPatch { status, .. })) |
                                            Some(LauncherState::XluaPatchAvailable(XluaPatch { status, .. })) => match status {
                                                PatchStatus::NotAvailable |
                                                PatchStatus::Outdated { .. } |
                                                PatchStatus::Preparation { .. } => &["error", "pill"],

                                                PatchStatus::Testing { .. } => &["warning", "pill"],
                                                PatchStatus::Available { .. } => &["suggested-action", "pill"]
                                            },

                                            Some(_) => &["suggested-action", "pill"],

                                            None => &["pill"]
                                        },

                                        #[watch]
                                        set_tooltip_text: Some(&match &model.state {
                                            Some(LauncherState::GameOutdated { .. }) |
                                            Some(LauncherState::VoiceOutdated(_)) => tr("main-window--version-outdated-tooltip"),

                                            Some(LauncherState::FolderMigrationRequired { .. }) => tr("migrate-folders-tooltip"),

                                            Some(LauncherState::UnityPlayerPatchAvailable(UnityPlayerPatch { status, .. })) |
                                            Some(LauncherState::XluaPatchAvailable(XluaPatch { status, .. })) => match status {
                                                PatchStatus::NotAvailable => tr("main-window--patch-unavailable-tooltip"),

                                                PatchStatus::Outdated { .. } |
                                                PatchStatus::Preparation { .. } => tr("main-window--patch-outdated-tooltip"),

                                                _ => String::new()
                                            },

                                            _ => String::new()
                                        }),

                                        set_hexpand: false,
                                        set_width_request: 200,

                                        connect_clicked => AppMsg::PerformAction
                                    }
                                },

                                adw::Bin {
                                    set_css_classes: &["background", "round-bin"],

                                    gtk::Button {
                                        #[watch]
                                        set_sensitive: !model.disabled_buttons,

                                        set_width_request: 44,

                                        add_css_class: "circular",
                                        set_icon_name: "emblem-system-symbolic",

                                        connect_clicked => AppMsg::OpenPreferences
                                    }
                                }
                            }
                        }
                    }
                }
            },

            connect_close_request[sender] => move |_| {
                if let Err(err) = Config::flush() {
                    sender.input(AppMsg::Toast {
                        title: tr("config-update-error"),
                        description: Some(err.to_string())
                    });
                }

                gtk::Inhibit::default()
            }
        }
    }

    fn init(
        _init: Self::Init,
        root: &Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        tracing::info!("Initializing main window");

        let model = App {
            progress_bar: ProgressBar::builder()
                .launch(ProgressBarInit {
                    caption: None,
                    display_progress: true,
                    display_fraction: true,
                    visible: true
                })
                .detach(),

            toast_overlay: adw::ToastOverlay::new(),

            loading: Some(None),
            style: CONFIG.launcher.style,
            state: None,

            downloading: false,
            disabled_buttons: false
        };

        model.progress_bar.widget().set_halign(gtk::Align::Center);
        model.progress_bar.widget().set_width_request(360);

        let toast_overlay = &model.toast_overlay;

        let widgets = view_output!();

        let about_dialog_broker: MessageBroker<AboutDialogMsg> = MessageBroker::new();

        unsafe {
            MAIN_WINDOW = Some(widgets.main_window.clone());

            PREFERENCES_WINDOW = Some(PreferencesApp::builder()
                .launch(widgets.main_window.clone().into())
                .forward(sender.input_sender(), std::convert::identity));

            ABOUT_DIALOG = Some(AboutDialog::builder()
                .transient_for(widgets.main_window.clone())
                .launch_with_broker((), &about_dialog_broker)
                .detach());
        }

        let mut group = RelmActionGroup::<WindowActionGroup>::new();

        // TODO: reduce code somehow

        group.add_action::<LauncherFolder>(RelmAction::new_stateless(clone!(@strong sender => move |_| {
            if let Err(err) = open::that(LAUNCHER_FOLDER.as_path()) {
                sender.input(AppMsg::Toast {
                    title: tr("launcher-folder-opening-error"),
                    description: Some(err.to_string())
                });

                tracing::error!("Failed to open launcher folder: {err}");
            }
        })));

        group.add_action::<GameFolder>(RelmAction::new_stateless(clone!(@strong sender => move |_| {
            let path = match Config::get() {
                Ok(config) => config.game.path.for_edition(config.launcher.edition).to_path_buf(),
                Err(_) => CONFIG.game.path.for_edition(CONFIG.launcher.edition).to_path_buf(),
            };

            if let Err(err) = open::that(path) {
                sender.input(AppMsg::Toast {
                    title: tr("game-folder-opening-error"),
                    description: Some(err.to_string())
                });

                tracing::error!("Failed to open game folder: {err}");
            }
        })));

        group.add_action::<ConfigFile>(RelmAction::new_stateless(clone!(@strong sender => move |_| {
            if let Ok(file) = config_file() {
                if let Err(err) = open::that(file) {
                    sender.input(AppMsg::Toast {
                        title: tr("config-file-opening-error"),
                        description: Some(err.to_string())
                    });

                    tracing::error!("Failed to open config file: {err}");
                }
            }
        })));

        group.add_action::<DebugFile>(RelmAction::new_stateless(clone!(@strong sender => move |_| {
            if let Err(err) = open::that(crate::DEBUG_FILE.as_os_str()) {
                sender.input(AppMsg::Toast {
                    title: tr("debug-file-opening-error"),
                    description: Some(err.to_string())
                });

                tracing::error!("Failed to open debug file: {err}");
            }
        })));

        group.add_action::<WishUrl>(RelmAction::new_stateless(clone!(@strong sender => move |_| {
            std::thread::spawn(clone!(@strong sender => move || {
                let config = Config::get().unwrap_or_else(|_| CONFIG.clone());

                let web_cache = config.game.path.for_edition(config.launcher.edition)
                    .join(config.launcher.edition.data_folder())
                    .join("webCaches/Cache/Cache_Data/data_2");

                if !web_cache.exists() {
                    tracing::error!("Couldn't find wishes URL: cache file doesn't exist");

                    sender.input(AppMsg::Toast {
                        title: tr("wish-url-search-failed"),
                        description: None
                    });
                }

                else {
                    match std::fs::read(&web_cache) {
                        Ok(web_cache) => {
                            let web_cache = String::from_utf8_lossy(&web_cache);

                            // https://webstatic-sea.[ho-yo-ver-se].com/[ge-nsh-in]/event/e20190909gacha-v2/index.html?......
                            if let Some(url) = web_cache.lines().rev().find(|line| line.contains("gacha-v2/index.html")) {
                                let url_begin_pos = url.find("https://").unwrap();
                                let url_end_pos = url_begin_pos + url[url_begin_pos..].find("\0\0\0\0").unwrap();

                                if let Err(err) = open::that(format!("{}#/log", &url[url_begin_pos..url_end_pos])) {
                                    tracing::error!("Failed to open wishes URL: {err}");
    
                                    sender.input(AppMsg::Toast {
                                        title: tr("wish-url-opening-error"),
                                        description: Some(err.to_string())
                                    });
                                }
                            }

                            else {
                                tracing::error!("Couldn't find wishes URL: no url found");

                                sender.input(AppMsg::Toast {
                                    title: tr("wish-url-search-failed"),
                                    description: None
                                });
                            }
                        }

                        Err(err) => {
                            tracing::error!("Couldn't find wishes URL: failed to open cache file: {err}");

                            sender.input(AppMsg::Toast {
                                title: tr("wish-url-search-failed"),
                                description: Some(err.to_string())
                            });
                        }
                    }
                }
            }));
        })));

        group.add_action::<About>(RelmAction::new_stateless(move |_| {
            about_dialog_broker.send(AboutDialogMsg::Show);
        }));

        widgets.main_window.insert_action_group("win", Some(&group.into_action_group()));

        tracing::info!("Main window initialized");

        let download_picture = model.style == LauncherStyle::Classic && !KEEP_BACKGROUND_FILE.exists();

        // Initialize some heavy tasks
        std::thread::spawn(move || {
            tracing::info!("Initializing heavy tasks");

            // Download background picture if needed

            if download_picture {
                sender.input(AppMsg::SetLoadingStatus(Some(Some(tr("downloading-background-picture")))));

                if let Err(err) = crate::background::download_background() {
                    tracing::error!("Failed to download background picture: {err}");

                    sender.input(AppMsg::Toast {
                        title: tr("background-downloading-failed"),
                        description: Some(err.to_string())
                    });
                }
            }

            // Update components index

            sender.input(AppMsg::SetLoadingStatus(Some(Some(tr("updating-components-index")))));

            let components = ComponentsLoader::new(&CONFIG.components.path);

            match components.is_sync(&CONFIG.components.servers) {
                Ok(Some(_)) => (),

                Ok(None) => {
                    for host in &CONFIG.components.servers {
                        match components.sync(host) {
                            Ok(changes) => {
                                sender.input(AppMsg::Toast {
                                    title: tr("components-index-updated"),
                                    description: if changes.is_empty() {
                                        None
                                    } else {
                                        Some(changes.into_iter()
                                            .map(|line| format!("- {line}"))
                                            .collect::<Vec<_>>()
                                            .join("\n"))
                                    }
                                });

                                break;
                            }

                            Err(err) => {
                                tracing::error!("Failed to sync components index");

                                sender.input(AppMsg::Toast {
                                    title: tr("components-index-sync-failed"),
                                    description: Some(err.to_string())
                                });
                            }
                        }
                    }
                }

                Err(err) => {
                    tracing::error!("Failed to verify that components index synced");

                    sender.input(AppMsg::Toast {
                        title: tr("components-index-verify-failed"),
                        description: Some(err.to_string())
                    });
                }
            }

            // Update initial patch status

            sender.input(AppMsg::SetLoadingStatus(Some(Some(tr("loading-patch-status")))));

            // Sync local patch repo
            let patch = Patch::new(&CONFIG.patch.path, CONFIG.launcher.edition);

            match patch.is_sync(&CONFIG.patch.servers) {
                Ok(Some(_)) => (),

                Ok(None) => {
                    for server in &CONFIG.patch.servers {
                        match patch.sync(server) {
                            Ok(_) => break,

                            Err(err) => {
                                tracing::error!("Failed to sync patch folder with remote: {server}: {err}");

                                sender.input(AppMsg::Toast {
                                    title: tr("patch-sync-failed"),
                                    description: Some(err.to_string())
                                });
                            }
                        }
                    }
                }

                Err(err) => {
                    tracing::error!("Failed to compare local patch folder with remote: {err}");

                    sender.input(AppMsg::Toast {
                        title: tr("patch-state-check-failed"),
                        description: Some(err.to_string())
                    });
                }
            }

            // Get main UnityPlayer patch status
            sender.input(AppMsg::SetUnityPlayerPatch(match patch.unity_player_patch() {
                Ok(patch) => Some(patch),

                Err(err) => {
                    tracing::error!("Failed to fetch unity player patch info: {err}");

                    sender.input(AppMsg::Toast {
                        title: tr("patch-info-fetching-error"),
                        description: Some(err.to_string())
                    });

                    None
                }
            }));

            // Get additional xlua patch status
            sender.input(AppMsg::SetXluaPatch(match patch.xlua_patch() {
                Ok(patch) => Some(patch),

                Err(err) => {
                    tracing::error!("Failed to fetch xlua patch info: {err}");

                    sender.input(AppMsg::Toast {
                        title: tr("patch-info-fetching-error"),
                        description: Some(err.to_string())
                    });

                    None
                }
            }));

            tracing::info!("Updated patch status");

            // Update initial game version status

            sender.input(AppMsg::SetLoadingStatus(Some(Some(tr("loading-game-version")))));

            sender.input(AppMsg::SetGameDiff(match GAME.try_get_diff() {
                Ok(diff) => Some(diff),
                Err(err) => {
                    tracing::error!("Failed to find game diff: {err}");

                    sender.input(AppMsg::Toast {
                        title: tr("game-diff-finding-error"),
                        description: Some(err.to_string())
                    });

                    None
                }
            }));

            tracing::info!("Updated game version status");

            // Update launcher state
            sender.input(AppMsg::UpdateLauncherState {
                perform_on_download_needed: false,
                apply_patch_if_needed: false,
                show_status_page: true
            });

            // Mark app as loaded
            unsafe {
                crate::READY = true;
            }

            tracing::info!("App is ready");
        });

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        tracing::debug!("Called main window event: {:?}", msg);

        match msg {
            // TODO: make function from this message like with toast
            AppMsg::UpdateLauncherState { perform_on_download_needed, apply_patch_if_needed, show_status_page } => {
                if show_status_page {
                    sender.input(AppMsg::SetLoadingStatus(Some(Some(tr("loading-launcher-state")))));
                } else {
                    self.disabled_buttons = true;
                }

                let updater = clone!(@strong sender => move |state| {
                    if show_status_page {
                        match state {
                            StateUpdating::Game => {
                                sender.input(AppMsg::SetLoadingStatus(Some(Some(tr("loading-launcher-state--game")))));
                            }

                            StateUpdating::Voice(locale) => {
                                sender.input(AppMsg::SetLoadingStatus(Some(Some(tr_args("loading-launcher-state--voice", [
                                    ("locale", locale.to_name().to_owned().into())
                                ])))));
                            }

                            StateUpdating::Patch => {
                                sender.input(AppMsg::SetLoadingStatus(Some(Some(tr("loading-launcher-state--patch")))));
                            }
                        }
                    }
                });

                let state = match LauncherState::get_from_config(updater) {
                    Ok(state) => Some(state),
                    Err(err) => {
                        tracing::error!("Failed to update launcher state: {err}");

                        self.toast(tr("launcher-state-updating-error"), Some(err.to_string()));
    
                        None
                    }
                };

                sender.input(AppMsg::SetLauncherState(state.clone()));

                if show_status_page {
                    sender.input(AppMsg::SetLoadingStatus(None));
                } else {
                    self.disabled_buttons = false;
                }
                
                if let Some(state) = state {
                    match state {
                        LauncherState::VoiceUpdateAvailable(_) |
                        LauncherState::VoiceNotInstalled(_) |
                        LauncherState::GameUpdateAvailable(_) |
                        LauncherState::GameNotInstalled(_) if perform_on_download_needed => {
                            sender.input(AppMsg::PerformAction);
                        }

                        LauncherState::UnityPlayerPatchAvailable(_) |
                        LauncherState::XluaPatchAvailable(_) if apply_patch_if_needed => {
                            sender.input(AppMsg::PerformAction);
                        }

                        _ => ()
                    }
                }
            }

            #[allow(unused_must_use)]
            AppMsg::SetGameDiff(diff) => unsafe {
                PREFERENCES_WINDOW.as_ref().unwrap_unchecked().sender().send(PreferencesAppMsg::SetGameDiff(diff));
            }

            #[allow(unused_must_use)]
            AppMsg::SetUnityPlayerPatch(patch) => unsafe {
                PREFERENCES_WINDOW.as_ref().unwrap_unchecked().sender().send(PreferencesAppMsg::SetUnityPlayerPatch(patch));
            }

            #[allow(unused_must_use)]
            AppMsg::SetXluaPatch(patch) => unsafe {
                PREFERENCES_WINDOW.as_ref().unwrap_unchecked().sender().send(PreferencesAppMsg::SetXluaPatch(patch));
            }

            AppMsg::SetLauncherState(state) => {
                self.state = state;
            }

            AppMsg::SetLoadingStatus(status) => {
                self.loading = status;
            }

            AppMsg::SetLauncherStyle(style) => {
                self.style = style;
            }

            AppMsg::SetDownloading(state) => {
                self.downloading = state;
            }

            AppMsg::DisableButtons(state) => {
                self.disabled_buttons = state;
            }

            AppMsg::OpenPreferences => unsafe {
                PREFERENCES_WINDOW.as_ref().unwrap_unchecked().widget().present();
            }

            AppMsg::RepairGame => repair_game::repair_game(sender, self.progress_bar.sender().to_owned()),

            #[allow(unused_must_use)]
            AppMsg::PredownloadUpdate => {
                if let Some(LauncherState::PredownloadAvailable { game, mut voices }) = self.state.clone() {
                    let tmp = Config::get().unwrap().launcher.temp.unwrap_or_else(std::env::temp_dir);

                    self.downloading = true;

                    let progress_bar_input = self.progress_bar.sender().clone();

                    progress_bar_input.send(ProgressBarMsg::UpdateCaption(Some(tr("downloading"))));

                    let mut diffs: Vec<VersionDiff> = vec![game];

                    diffs.append(&mut voices);

                    std::thread::spawn(move || {
                        for mut diff in diffs {
                            let result = diff.download_to(&tmp, clone!(@strong progress_bar_input => move |curr, total| {
                                progress_bar_input.send(ProgressBarMsg::UpdateProgress(curr, total));
                            }));

                            if let Err(err) = result {
                                sender.input(AppMsg::Toast {
                                    title: tr("downloading-failed"),
                                    description: Some(err.to_string())
                                });

                                tracing::error!("Failed to predownload update: {err}");

                                break;
                            }
                        }

                        sender.input(AppMsg::SetDownloading(false));
                        sender.input(AppMsg::UpdateLauncherState {
                            perform_on_download_needed: false,
                            apply_patch_if_needed: false,
                            show_status_page: true
                        });
                    });
                }
            }

            AppMsg::PerformAction => unsafe {
                match self.state.as_ref().unwrap_unchecked() {
                    LauncherState::UnityPlayerPatchAvailable(UnityPlayerPatch { status: PatchStatus::NotAvailable, .. }) |
                    LauncherState::XluaPatchAvailable(XluaPatch { status: PatchStatus::NotAvailable, .. }) |
                    LauncherState::PredownloadAvailable { .. } |
                    LauncherState::Launch => launch::launch(sender),

                    LauncherState::FolderMigrationRequired { from, to, cleanup_folder } =>
                        migrate_folder::migrate_folder(sender, from.to_owned(), to.to_owned(), cleanup_folder.to_owned()),

                    LauncherState::UnityPlayerPatchAvailable(patch) => apply_patch::apply_patch(sender, patch.to_owned()),
                    LauncherState::XluaPatchAvailable(patch) => apply_patch::apply_patch(sender, patch.to_owned()),

                    LauncherState::WineNotInstalled => download_wine::download_wine(sender, self.progress_bar.sender().to_owned()),

                    LauncherState::PrefixNotExists => create_prefix::create_prefix(sender),

                    LauncherState::VoiceUpdateAvailable(diff) |
                    LauncherState::VoiceNotInstalled(diff) |
                    LauncherState::GameUpdateAvailable(diff) |
                    LauncherState::GameNotInstalled(diff) =>
                        download_diff::download_diff(sender, self.progress_bar.sender().to_owned(), diff.to_owned()),

                    LauncherState::VoiceOutdated(_) |
                    LauncherState::GameOutdated(_) => ()
                }
            }

            AppMsg::HideWindow => unsafe {
                MAIN_WINDOW.as_ref().unwrap_unchecked().set_visible(false);
            }

            AppMsg::ShowWindow => unsafe {
                MAIN_WINDOW.as_ref().unwrap_unchecked().present();
            }

            AppMsg::Toast { title, description } => self.toast(title, description)
        }
    }
}

impl App {
    pub fn toast<T: AsRef<str>>(&mut self, title: T, description: Option<T>) {
        let toast = adw::Toast::new(title.as_ref());

        toast.set_timeout(4);

        if let Some(description) = description {
            toast.set_button_label(Some(&tr("details")));

            let dialog = adw::MessageDialog::new(
                Some(unsafe { MAIN_WINDOW.as_ref().unwrap_unchecked() }),
                Some(title.as_ref()),
                Some(description.as_ref())
            );

            dialog.add_response("close", &tr("close"));
            dialog.add_response("save", &tr("save"));

            dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);

            dialog.connect_response(Some("save"), |_, _| {
                if let Err(err) = open::that(crate::DEBUG_FILE.as_os_str()) {
                    tracing::error!("Failed to open debug file: {err}");
                }
            });

            toast.connect_button_clicked(move |_| {
                dialog.present();
            });
        }

        self.toast_overlay.add_toast(toast);
    }
}

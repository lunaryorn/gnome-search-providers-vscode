// Copyright Sebastian Wiesner <sebastian@swsnr.de>
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

#![deny(warnings, missing_docs, clippy::all)]

//! Gnome search provider for VSCode editors.

use std::convert::TryFrom;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Error, Result};
use async_trait::async_trait;
use log::{error, info, trace, warn};
use serde::Deserialize;

use gnome_search_provider_common::app::*;
use gnome_search_provider_common::dbus::*;
use gnome_search_provider_common::futures_channel;
use gnome_search_provider_common::gio;
use gnome_search_provider_common::gio::glib;
use gnome_search_provider_common::gio::prelude::*;
use gnome_search_provider_common::log::*;
use gnome_search_provider_common::mainloop::*;
use gnome_search_provider_common::matching::*;
use gnome_search_provider_common::source::{AsyncItemsSource, IdMap};
use gnome_search_provider_common::zbus;
use gnome_search_provider_common::zbus::names::WellKnownName;

#[derive(Debug, Deserialize)]
struct WorkspaceEntry {
    #[serde(rename = "configPath")]
    config_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum StorageOpenedPathsListEntry {
    Workspace {
        workspace: WorkspaceEntry,
    },
    Folder {
        #[serde(rename = "folderUri")]
        uri: String,
    },
    Other(serde_json::Value),
}

impl StorageOpenedPathsListEntry {
    /// Move this entry into a workspace URL.
    fn into_workspace_url(self) -> Option<String> {
        match self {
            Self::Workspace { workspace } => Some(workspace.config_path),
            Self::Folder { uri } => Some(uri),
            Self::Other(_) => None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct StorageOpenedPathsList {
    /// Up to code 1.54
    workspaces3: Option<Vec<String>>,
    /// From code 1.55
    entries: Option<Vec<StorageOpenedPathsListEntry>>,
}

#[derive(Debug, Deserialize)]
struct Storage {
    #[serde(rename = "openedPathsList")]
    opened_paths_list: Option<StorageOpenedPathsList>,
}

impl Storage {
    /// Read a VSCode storage.json from the given `reader`.
    fn read<R: Read>(reader: R) -> Result<Self> {
        serde_json::from_reader(reader).map_err(Into::into)
    }

    /// Read the `storage.json` file in the given `config_dir`.
    async fn from_dir<P: AsRef<Path>>(config_dir: P) -> Result<Self> {
        let path = config_dir.as_ref().join("storage.json");
        trace!("Reading storage from {}", path.display());
        let (data, _) = gio::File::for_path(&path)
            .load_contents_async_future()
            .await
            .with_context(|| format!("Failed to read storage data from {}", path.display()))?;
        Self::read(data.as_slice())
            .with_context(|| format!("Failed to parse storage from {}", path.display()))
    }

    /// Move this storage into workspace URLs.
    fn into_workspace_urls(self) -> Vec<String> {
        trace!("Extracting workspace URLs from {:?}", self);
        if let Some(paths) = self.opened_paths_list {
            let entries = paths.entries.unwrap_or_default();
            let workspaces3 = paths.workspaces3.unwrap_or_default();
            entries
                .into_iter()
                .filter_map(|entry| entry.into_workspace_url())
                .chain(workspaces3.into_iter())
                .collect()
        } else {
            Vec::new()
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct ConfigLocation<'a> {
    dirname: &'a str,
}

/// A search provider to expose from this service.
struct ProviderDefinition<'a> {
    /// A human readable label for this provider.
    label: &'a str,
    /// The ID (that is, the filename) of the desktop file of the corresponding app.
    desktop_id: &'a str,
    /// The relative object path to expose this provider at.
    relative_obj_path: &'a str,
    /// The location of the configuration for this app.
    config: ConfigLocation<'a>,
}

impl ProviderDefinition<'_> {
    /// Gets the full object path for this provider.
    fn objpath(&self) -> String {
        format!("/de/swsnr/searchprovider/vscode/{}", self.relative_obj_path)
    }
}

/// Known search providers.
///
/// For each definition in this array a corresponding provider file must exist in
/// `providers/`; the file must refer to the same `desktop_id` and the same object path.
/// The object path must be unique for each desktop ID, to ensure that this service always
/// launches the right application associated with the search provider.
const PROVIDERS: &[ProviderDefinition] = &[
    // The standard Arch Linux code package from community
    ProviderDefinition {
        label: "Code OSS (Arch Linux)",
        desktop_id: "code-oss.desktop",
        relative_obj_path: "arch/codeoss",
        config: ConfigLocation {
            dirname: "Code - OSS",
        },
    },
    // The binary AUR package for visual studio code: https://aur.archlinux.org/packages/visual-studio-code-bin/
    ProviderDefinition {
        label: "Visual Studio Code (AUR package)",
        desktop_id: "visual-studio-code.desktop",
        relative_obj_path: "aur/visualstudiocode",
        config: ConfigLocation { dirname: "Code" },
    },
    // The standard codium package on Linux from here: https://github.com/VSCodium/vscodium.
    // Should work for most Linux distributions packaged from here.
    ProviderDefinition {
        label: "VSCodium",
        desktop_id: "codium.desktop",
        relative_obj_path: "codium",
        config: ConfigLocation {
            dirname: "VSCodium",
        },
    },
    // The official install packages from https://code.visualstudio.com/download.
    ProviderDefinition {
        label: "Visual Studio Code (Official package)",
        desktop_id: "code.desktop",
        relative_obj_path: "official/code",
        config: ConfigLocation { dirname: "Code" },
    },
];

/// A recent workspace of a VSCode variant.
#[derive(Debug, PartialEq)]
struct RecentWorkspace {
    /// The human readable nfame.
    name: String,
    /// The workspace URL.
    url: String,
}

fn recent_item(url: String) -> Result<AppLaunchItem> {
    if let Some(name) = url.split('/').last() {
        let item = AppLaunchItem {
            name: name.to_string(),
            uri: url,
        };
        trace!("Found recent workspace item {:?}", item);
        Ok(item)
    } else {
        Err(anyhow!("Failed to extract workspace name from URL {}", url))
    }
}

struct VscodeWorkspacesSource {
    app_id: AppId,
    /// The configuration directory.
    config_dir: PathBuf,
}

#[async_trait]
impl AsyncItemsSource<AppLaunchItem> for VscodeWorkspacesSource {
    type Err = Error;

    async fn find_recent_items(&self) -> Result<IdMap<AppLaunchItem>, Self::Err> {
        info!("Finding recent workspaces for {}", self.app_id);
        // Move to the main thread and then asynchronously read recent items through Gio,
        // and get them sent back to us via a oneshot channel.
        let (send, recv) = futures_channel::oneshot::channel();
        let dir = self.config_dir.clone();
        glib::MainContext::default().invoke(move || {
            glib::MainContext::default()
                .spawn_local(async move { send.send(Storage::from_dir(dir).await).unwrap() });
        });

        let urls = recv.await.unwrap()?.into_workspace_urls();
        let mut items = IndexMap::new();
        for url in urls {
            trace!("Discovered workspace url {}", url);
            let id = format!("vscode-search-provider-{}-{}", self.app_id, &url);
            match recent_item(url) {
                Ok(item) => {
                    items.insert(id, item);
                }
                Err(err) => {
                    warn!("Skipping workspace: {}", err)
                }
            }
        }
        info!("Found {} workspace(s) for {}", items.len(), self.app_id);
        Ok(items)
    }
}

/// The name to request on the bus.
const BUSNAME: &str = "de.swsnr.searchprovider.VSCode";

async fn register_search_providers(
    connection: &zbus::Connection,
    launch_service: &AppLaunchService,
) -> Result<()> {
    let user_config_dir = glib::user_config_dir();
    let mut object_server = connection.object_server_mut().await;
    for provider in PROVIDERS {
        if let Some(app) = gio::DesktopAppInfo::new(provider.desktop_id) {
            info!(
                "Registering provider for {} at {}",
                provider.desktop_id,
                provider.objpath()
            );
            let dbus_provider = AppItemSearchProvider::new(
                app.into(),
                VscodeWorkspacesSource {
                    app_id: provider.desktop_id.into(),
                    config_dir: user_config_dir.join(provider.config.dirname),
                },
                launch_service.client(),
            );
            object_server.at(provider.objpath().as_str(), dbus_provider)?;
        }
    }
    Ok(())
}

async fn tick(connection: zbus::Connection) {
    loop {
        connection.executor().tick().await
    }
}

/// Starts the DBUS service loop.
///
/// Connect to the ession bus and register DBus objects for every provider
/// whose underlying VSCode variant is installed.
///
/// Then register the connection on the Glib main loop and handle incoming messages.
async fn start_dbus_service() -> Result<()> {
    let connection = zbus::ConnectionBuilder::session()?
        // We run on the glib mainloop, and avoid the separate thread
        .internal_executor(false)
        .build()
        .await
        .with_context(|| "Failed to connect to session bus")?;

    glib::MainContext::ref_thread_default().spawn(tick(connection.clone()));

    info!("Registering all search providers");
    let launch_service = AppLaunchService::new(
        &glib::MainContext::ref_thread_default(),
        connection.clone(),
        SystemdScopeSettings {
            prefix: concat!("app-", env!("CARGO_BIN_NAME")).to_string(),
            started_by: env!("CARGO_BIN_NAME").to_string(),
            documentation: vec![env!("CARGO_PKG_HOMEPAGE").to_string()],
        },
    );
    register_search_providers(&connection, &launch_service).await?;

    info!("All providers registered, acquiring {}", BUSNAME);
    // Work around https://gitlab.freedesktop.org/dbus/zbus/-/issues/199,
    // remove once https://gitlab.freedesktop.org/dbus/zbus/-/merge_requests/414 is merged and released
    request_name_exclusive(&connection, WellKnownName::try_from(BUSNAME).unwrap())
        .await
        .with_context(|| format!("Failed to request {}", BUSNAME))?;

    info!("Acquired name {}, serving search providers", BUSNAME);
    Ok(())
}

fn app() -> clap::App<'static> {
    use clap::*;
    app_from_crate!()
        .setting(AppSettings::DontCollapseArgsInUsage)
        .setting(AppSettings::DeriveDisplayOrder)
        .term_width(80)
        .after_help(
            "\
Set $RUST_LOG to control the log level",
        )
        .arg(
            Arg::new("providers")
                .long("--providers")
                .help("List all providers"),
        )
        .arg(
            Arg::new("journal_log")
                .long("--journal-log")
                .help("Directly log to the systemd journal instead of stdout"),
        )
}

fn main() {
    let matches = app().get_matches();
    if matches.is_present("providers") {
        let mut labels: Vec<&'static str> = PROVIDERS.iter().map(|p| p.label).collect();
        labels.sort_unstable();
        for label in labels {
            println!("{}", label)
        }
    } else {
        setup_logging_for_service(env!("CARGO_PKG_VERSION"));

        info!(
            "Started {} version: {}",
            env!("CARGO_BIN_NAME"),
            env!("CARGO_PKG_VERSION")
        );

        trace!("Acquire main context");
        let context = glib::MainContext::default();
        context.push_thread_default();

        if let Err(error) = context.block_on(start_dbus_service()) {
            error!("Failed to start DBus server: {}", error);
            std::process::exit(1);
        } else {
            create_main_loop(&context).run();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::app;
    use crate::Storage;

    #[test]
    fn verify_app() {
        app().debug_assert();
    }

    #[test]
    fn read_recent_workspaces_code_1_54() {
        let data: &[u8] = include_bytes!("tests/code_1_54_storage.json");
        let storage = Storage::read(data).unwrap();
        assert!(
            &storage.opened_paths_list.is_some(),
            "opened paths list missing"
        );
        assert!(
            &storage
                .opened_paths_list
                .as_ref()
                .unwrap()
                .workspaces3
                .is_some(),
            "workspaces3 missing"
        );
        assert_eq!(
            storage.into_workspace_urls(),
            vec![
                "file:///home/foo//mdcat",
                "file:///home/foo//gnome-jetbrains-search-provider",
                "file:///home/foo//gnome-shell",
                "file:///home/foo//sbctl",
            ]
        )
    }

    #[test]
    fn read_recent_workspaces_code_1_55() {
        let data: &[u8] = include_bytes!("tests/code_1_55_storage.json");
        let storage = Storage::read(data).unwrap();
        assert!(
            &storage.opened_paths_list.is_some(),
            "opened paths list missing"
        );
        assert!(
            &storage
                .opened_paths_list
                .as_ref()
                .unwrap()
                .entries
                .is_some(),
            "entries missing"
        );

        assert_eq!(
            storage.into_workspace_urls(),
            vec![
                "file:///home/foo//workspace.code-workspace",
                "file:///home/foo//mdcat",
                "file:///home/foo//gnome-jetbrains-search-provider",
                "file:///home/foo//gnome-shell",
                "file:///home/foo//sbctl",
            ]
        );
    }

    mod providers {
        use crate::{BUSNAME, PROVIDERS};
        use anyhow::{Context, Result};
        use ini::Ini;
        use std::collections::HashSet;
        use std::path::Path;

        struct ProviderFile {
            desktop_id: String,
            object_path: String,
            bus_name: String,
            version: String,
        }

        fn load_all_provider_files() -> Result<Vec<ProviderFile>> {
            let mut providers = Vec::new();
            let ini_files = globwalk::GlobWalkerBuilder::new(
                Path::new(env!("CARGO_MANIFEST_DIR")).join("providers"),
                "*.ini",
            )
            .build()
            .unwrap();
            for entry in ini_files {
                let filepath = entry.unwrap().into_path();
                let ini = Ini::load_from_file(&filepath).with_context(|| {
                    format!("Failed to parse ini file at {}", filepath.display())
                })?;
                let provider = ProviderFile {
                    desktop_id: ini
                        .get_from(Some("Shell Search Provider"), "DesktopId")
                        .with_context(|| format!("DesktopId missing in {}", &filepath.display()))?
                        .to_string(),
                    object_path: ini
                        .get_from(Some("Shell Search Provider"), "ObjectPath")
                        .with_context(|| format!("ObjectPath missing in {}", &filepath.display()))?
                        .to_string(),
                    bus_name: ini
                        .get_from(Some("Shell Search Provider"), "BusName")
                        .with_context(|| format!("BusName missing in {}", &filepath.display()))?
                        .to_string(),
                    version: ini
                        .get_from(Some("Shell Search Provider"), "Version")
                        .with_context(|| format!("Version missing in {}", &filepath.display()))?
                        .to_string(),
                };
                providers.push(provider);
            }

            Ok(providers)
        }

        #[test]
        fn all_providers_have_a_correct_ini_file() {
            let provider_files = load_all_provider_files().unwrap();
            for provider in PROVIDERS {
                let provider_file = provider_files
                    .iter()
                    .find(|p| p.desktop_id == provider.desktop_id);
                assert!(
                    provider_file.is_some(),
                    "Provider INI missing for provider {} with desktop ID {}",
                    provider.label,
                    provider.desktop_id
                );

                assert_eq!(provider_file.unwrap().object_path, provider.objpath());
                assert_eq!(provider_file.unwrap().bus_name, BUSNAME);
                assert_eq!(provider_file.unwrap().version, "2");
            }
        }

        #[test]
        fn no_extra_ini_files_without_providers() {
            let provider_files = load_all_provider_files().unwrap();
            assert_eq!(PROVIDERS.len(), provider_files.len());
        }

        #[test]
        fn desktop_ids_are_unique() {
            let mut ids = HashSet::new();
            for provider in PROVIDERS {
                ids.insert(provider.desktop_id);
            }
            assert_eq!(PROVIDERS.len(), ids.len());
        }

        #[test]
        fn dbus_paths_are_unique() {
            let mut paths = HashSet::new();
            for provider in PROVIDERS {
                paths.insert(provider.objpath());
            }
            assert_eq!(PROVIDERS.len(), paths.len());
        }
    }
}

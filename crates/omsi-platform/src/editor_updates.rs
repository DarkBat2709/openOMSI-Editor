//! Independent update channels for the standalone editor and the separate base game.
//! The official self-updater remains disabled in editor builds.
use crate::updater::{self, Place, Release, Status};
use anyhow::{bail, Context};
use std::{
    io::Write,
    path::Path,
    sync::{Arc, Mutex},
};

pub const EDITOR_REPO: &str = "DarkBat2709/openOMSI-Editor";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    Game,
    Editor,
}
impl Channel {
    pub fn title(self) -> &'static str {
        match self {
            Self::Game => "openOMSI",
            Self::Editor => "openOMSI Editor",
        }
    }
    pub fn repo(self) -> &'static str {
        match self {
            Self::Game => updater::REPO,
            Self::Editor => EDITOR_REPO,
        }
    }
}
#[derive(Clone, Default)]
pub struct Installation {
    pub current: String,
    pub place: Option<Place>,
    pub installed: bool,
}
pub struct ManagedUpdate {
    pub channel: Channel,
    status: Arc<Mutex<Status>>,
    installation: Arc<Mutex<Installation>>,
    pub relaunched: bool,
}
fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}
impl ManagedUpdate {
    pub fn new(channel: Channel) -> Self {
        Self {
            channel,
            status: Arc::new(Mutex::new(Status::Idle)),
            installation: Arc::new(Mutex::new(Installation::default())),
            relaunched: false,
        }
    }
    pub fn fail(&self, message: String) {
        *lock(&self.status) = Status::Failed(message);
    }
    pub fn status(&self) -> Status {
        lock(&self.status).clone()
    }
    pub fn installation(&self) -> Installation {
        lock(&self.installation).clone()
    }
    pub fn busy(&self) -> bool {
        matches!(
            self.status(),
            Status::Checking
                | Status::Downloading { .. }
                | Status::Installing(_)
                | Status::Restarting(_)
        )
    }
    pub fn check(&mut self) {
        if self.busy() {
            return;
        }
        *lock(&self.status) = Status::Checking;
        let (channel, status, installation) =
            (self.channel, self.status.clone(), self.installation.clone());
        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<Option<Release>> {
                let i = locate(channel)?;
                let current = i.current.clone();
                *lock(&installation) = i;
                let url = format!(
                    "https://api.github.com/repos/{}/releases{}",
                    channel.repo(),
                    if channel == Channel::Game {
                        "/latest"
                    } else {
                        "?per_page=100"
                    }
                );
                let json = serde_json::from_str(&updater::fetch_text(&url)?)?;
                match channel {
                    Channel::Game => updater::parse_release(&json, &current, false),
                    Channel::Editor => editor_release(&json, &current),
                }
            })();
            *lock(&status) = match result {
                Ok(Some(r)) => Status::Available(r),
                Ok(None) => Status::UpToDate,
                Err(e) => Status::Failed(format!("{e:#}")),
            };
        });
    }
    pub fn install(&mut self, game_running: bool) {
        if game_running {
            *lock(&self.status) =
                Status::Failed("End the running session before installing an update.".into());
            return;
        }
        let Status::Available(r) = self.status() else {
            return;
        };
        let (channel, status, installation) =
            (self.channel, self.status.clone(), self.installation.clone());
        *lock(&status) = Status::Downloading {
            release: r.clone(),
            done: 0,
            total: r.size,
        };
        std::thread::spawn(move || {
            let result = (|| -> anyhow::Result<()> {
                let i = lock(&installation).clone();
                let p = i.place.context("No installation selected")?;
                validate_target(channel, &p, &std::env::current_exe()?)?;
                // Re-read before replacing anything: the installation may have changed since checking.
                let current = installed_version(channel, &p.exe)?;
                if !release_newer(&r.version, &current) {
                    bail!("This version is already installed or newer.");
                }
                let probe = p
                    .dir
                    .join(format!(".editor-update-write-{}", std::process::id()));
                let file_probe = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&probe)
                    .context("The installation folder is not writable")?;
                drop(file_probe);
                std::fs::remove_file(probe)?;
                let file = updater::download_dir().join(&r.asset_name);
                updater::download(&r, &file, &status)?;
                *lock(&status) = Status::Installing(r.clone());
                verify_checksum(&file, &r)?;
                ensure_not_running(channel, &p.exe)?;
                backup_program(&file, &p, channel, &current, &r.version)?;
                match channel {
                    Channel::Game => updater::install_archive(&file, &p)?,
                    Channel::Editor => install_editor_archive(&file, &p, &r.version)?,
                }
                // Keep the old files for recovery. Only the current editor is restarted.
                let mut installed = lock(&installation);
                installed.current = r.version.clone();
                installed.installed = true;
                *lock(&status) = if channel == Channel::Editor {
                    Status::Restarting(r)
                } else {
                    Status::UpToDate
                };
                Ok(())
            })();
            if let Err(e) = result {
                *lock(&status) = Status::Failed(format!("{e:#}"));
            }
        });
    }
    pub fn restart(&mut self) -> anyhow::Result<()> {
        if self.relaunched {
            return Ok(());
        }
        self.relaunched = true;
        let p = self
            .installation()
            .place
            .context("No editor installation selected")?;
        // Preserve content root, launch arguments and working directory of start-editor.sh.
        std::process::Command::new(p.exe)
            .args(std::env::args_os().skip(1))
            .spawn()?;
        Ok(())
    }
}

fn locate(channel: Channel) -> anyhow::Result<Installation> {
    let own = std::env::current_exe()?.canonicalize()?;
    let exe = match channel {
        Channel::Editor => {
            if own.file_stem().is_some_and(|n| n == "openomsi-launcher-app") {
                own.with_file_name(if cfg!(windows) { "openomsi.exe" } else { "openomsi" })
            } else { own.clone() }
        },
        Channel::Game => {
            let content = omsi_launcher_lib::content_dir()
                .context("No openOMSI content folder configured")?;
            let candidate = content.join(if cfg!(windows) {
                "openomsi.exe"
            } else {
                "openomsi"
            });
            if !candidate.is_file() {
                bail!("No separate openOMSI installation in {}. Install the official game there first.", content.display());
            }
            candidate.canonicalize()?
        }
    };
    let p = Place {
        dir: exe
            .parent()
            .context("No installation folder")?
            .to_path_buf(),
        bundle: None,
        exe,
    };
    validate_target(channel, &p, &own)?;
    let current = installed_version(channel, &p.exe)?;
    Ok(Installation {
        current,
        place: Some(p),
        installed: false,
    })
}
fn validate_target(channel: Channel, p: &Place, own: &Path) -> anyhow::Result<()> {
    if updater::is_dev_build(&p.exe) {
        bail!("Development builds must be updated with git and cargo.");
    }
    if channel == Channel::Game
        && (p.exe.canonicalize()? == own.canonicalize()?
            || p.dir.canonicalize()? == own.parent().context("No editor folder")?.canonicalize()?
            || p.dir.join("VERSION-EDITOR.txt").exists())
    {
        bail!("The official update needs a separate game installation; the editor folder is protected.");
    }
    if channel == Channel::Editor
        && (!cfg!(all(target_os = "linux", target_arch = "x86_64"))
            || !p.dir.join("VERSION-EDITOR.txt").is_file())
    {
        bail!("Automatic editor updates require the Linux x64 editor package with VERSION-EDITOR.txt.");
    }
    Ok(())
}
fn installed_version(channel: Channel, exe: &Path) -> anyhow::Result<String> {
    if channel == Channel::Editor {
        let text = std::fs::read_to_string(exe.parent().unwrap().join("VERSION-EDITOR.txt"))?;
        let versions: Vec<_> = text
            .lines()
            .filter_map(|l| l.strip_prefix("Editor "))
            .collect();
        if versions.len() != 1 || !valid_version(versions[0]) {
            bail!("VERSION-EDITOR.txt contains no unambiguous editor version.");
        }
        return Ok(versions[0].to_string());
    }
    let mut child = std::process::Command::new(exe)
        .arg("--version")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        if child.try_wait()?.is_some() {
            break;
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("The installed openOMSI did not report its version.");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let output = child.wait_with_output()?;
    let text = String::from_utf8(output.stdout)?;
    let version = text
        .trim()
        .strip_prefix("openomsi ")
        .context("Unknown openOMSI version output")?;
    if !output.status.success() || !valid_version(version) || version.contains("editor") {
        bail!("This program is not an official openOMSI installation.");
    }
    Ok(version.to_string())
}
fn valid_version(v: &str) -> bool {
    let number = v.split('-').next().unwrap_or("");
    number.split('.').count() == 3
        && number
            .split('.')
            .all(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".-+".contains(&b))
}
fn release_newer(candidate: &str, current: &str) -> bool {
    updater::newer(candidate, current)
        || (candidate.split('-').next() == current.split('-').next()
            && !candidate.contains('-')
            && current.contains('-'))
}
fn editor_release(json: &serde_json::Value, current: &str) -> anyhow::Result<Option<Release>> {
    if !cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        bail!("No automatic editor update package for this platform.");
    }
    let releases = json
        .as_array()
        .context("GitHub returned no editor release list")?;
    let mut best: Option<Release> = None;
    for v in releases {
        if v["draft"].as_bool() == Some(true) {
            continue;
        }
        let Some(tag) = v["tag_name"].as_str() else {
            continue;
        };
        let version = tag.trim_start_matches('v');
        if !valid_version(version) || !release_newer(version, current) {
            continue;
        }
        let want = format!("openOMSI-Editor-{version}-linux-x64.tar.gz");
        let Some(assets) = v["assets"].as_array() else {
            continue;
        };
        let Some(a) = assets.iter().find(|a| a["name"].as_str() == Some(&want)) else {
            continue;
        };
        let mut sha = a["digest"]
            .as_str()
            .and_then(|s| s.strip_prefix("sha256:"))
            .map(str::to_string);
        if sha.is_none() {
            if let Some(checksum) = assets
                .iter()
                .find(|a| a["name"].as_str() == Some(&(want.clone() + ".sha256")))
            {
                let text = updater::fetch_text(
                    checksum["browser_download_url"]
                        .as_str()
                        .context("No checksum URL")?,
                )?;
                sha = text.split_whitespace().next().map(str::to_string);
            }
        }
        let sha = sha
            .filter(|s| s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit()))
            .context("The editor release has no valid SHA-256 checksum")?;
        let r = Release {
            version: version.into(),
            page: format!("https://github.com/{EDITOR_REPO}/releases/tag/{tag}"),
            notes: v["body"].as_str().unwrap_or("").into(),
            asset_name: want,
            asset_url: a["browser_download_url"]
                .as_str()
                .context("No editor package URL")?
                .into(),
            size: a["size"].as_u64().unwrap_or(0),
            sha256: Some(sha.to_ascii_lowercase()),
        };
        if best
            .as_ref()
            .is_none_or(|b| release_newer(&r.version, &b.version))
        {
            best = Some(r);
        }
    }
    Ok(best)
}

/// Convert the editor tar package to the flat ZIP layout of the rollback installer.
/// Entries are copied as data; links, traversal and files outside the program package are refused.
fn editor_zip(archive: &Path, output: &Path, version: &str) -> anyhow::Result<()> {
    let input = flate2::read::GzDecoder::new(std::fs::File::open(archive)?);
    let mut tar = tar::Archive::new(input);
    let mut zip = zip::ZipWriter::new(std::fs::File::create(output)?);
    let prefix = format!("openOMSI-Editor-{version}");
    let mut names = std::collections::HashSet::new();
    let mut has_program = false;
    let mut has_version = false;
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        if path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            bail!("Unsafe editor archive path");
        }
        let relative = path
            .strip_prefix(&prefix)
            .context("Wrong editor package folder")?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        let top = relative
            .components()
            .next()
            .unwrap()
            .as_os_str()
            .to_str()
            .context("Invalid archive filename")?;
        if ![
            "openomsi",
            "openomsi-launcher-app",
            "libsteam_api.so",
            "start-editor.sh",
            "start-editor-diagnose.sh",
            "VERSION-EDITOR.txt",
            "assets",
            "docs",
        ]
        .contains(&top)
        {
            bail!("Unexpected editor package file: {}", relative.display());
        }
        let kind = entry.header().entry_type();
        if kind.is_dir() {
            continue;
        }
        if !kind.is_file() || !names.insert(relative.to_path_buf()) {
            bail!("Links or duplicate files in editor package");
        }
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(entry.header().mode()? & 0o777);
        zip.start_file(relative.to_str().context("Invalid archive path")?, opts)?;
        if relative == Path::new("VERSION-EDITOR.txt") {
            let mut text = String::new();
            std::io::Read::read_to_string(&mut entry, &mut text)?;
            if text
                .lines()
                .filter(|l| l.starts_with("Editor "))
                .collect::<Vec<_>>()
                != vec![format!("Editor {version}")]
            {
                bail!("Editor package version does not match release");
            }
            zip.write_all(text.as_bytes())?;
            has_version = true;
        } else {
            std::io::copy(&mut entry, &mut zip)?;
        }
        has_program |= relative == Path::new("openomsi");
    }
    if !has_program || !has_version {
        bail!("Incomplete editor package");
    }
    zip.finish()?;
    Ok(())
}
fn install_editor_archive(archive: &Path, p: &Place, version: &str) -> anyhow::Result<()> {
    let converted = archive.with_extension("install.zip");
    let result = editor_zip(archive, &converted, version)
        .and_then(|_| updater::install_archive(&converted, p));
    let _ = std::fs::remove_file(converted);
    result
}

fn verify_checksum(file: &Path, r: &Release) -> anyhow::Result<()> {
    use sha2::Digest;
    let expected = r
        .sha256
        .as_deref()
        .context("This update has no SHA-256 checksum")?;
    let mut hash = sha2::Sha256::new();
    use std::io::Read;
    let mut input = std::fs::File::open(file)?;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = input.read(&mut buffer)?;
        if n == 0 { break; }
        hash.update(&buffer[..n]);
    }
    let actual: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if !actual.eq_ignore_ascii_case(expected) {
        bail!("Update checksum mismatch; nothing has been installed.");
    }
    Ok(())
}
fn ensure_not_running(channel: Channel, exe: &Path) -> anyhow::Result<()> {
    #[cfg(target_os = "linux")]
    if channel == Channel::Game {
        let target = exe.canonicalize()?;
        for p in std::fs::read_dir("/proc")?.flatten() {
            if let Ok(running) = std::fs::read_link(p.path().join("exe")) {
                if running == target {
                    bail!("End the running openOMSI session before installing.");
                }
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = (channel, exe);
    Ok(())
}
/// Keep the replaced program files beyond the self-updater's startup cleanup.
/// Content directories can never be supplied as program files.
fn backup_program(
    archive: &Path,
    p: &Place,
    channel: Channel,
    current: &str,
    next: &str,
) -> anyhow::Result<()> {
    let zip_file = if channel == Channel::Editor {
        let f = archive.with_extension("install.zip");
        editor_zip(archive, &f, next)?;
        f
    } else {
        archive.to_path_buf()
    };
    let result = (|| -> anyhow::Result<()> {
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&zip_file)?)?;
        let mut names = std::collections::BTreeSet::new();
        for index in 0..zip.len() {
            let entry = zip.by_index(index)?;
            let path = entry
                .enclosed_name()
                .context("Unsafe update archive path")?;
            let name = path
                .components()
                .next()
                .context("Empty update archive path")?
                .as_os_str()
                .to_str()
                .context("Invalid update archive path")?
                .to_string();
            let lower = name.to_ascii_lowercase();
            if [
                "vehicles",
                "maps",
                "sceneryobjects",
                "splines",
                "texture",
                "humans",
                "fonts",
                "sounds",
                "tickets",
                "drivers",
            ]
            .contains(&lower.as_str())
                || name.starts_with('.')
                || lower.ends_with(".old-update")
            {
                bail!("The update archive contains content or reserved files: {name}");
            }
            if entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
                bail!("Links in update archive");
            }
            names.insert(name);
        }
        if !names.contains("openomsi") && !names.contains("openomsi.exe") {
            bail!("No program in update archive");
        }
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let backup = p
            .dir
            .join(".editor-update-backups")
            .join(format!("{current}-{stamp}"));
        std::fs::create_dir_all(&backup)?;
        for name in names {
            let original = p.dir.join(&name);
            if original.exists() {
                copy_tree(&original, &backup.join(name))?;
            }
        }
        // The manifest is part of recovery, too; start.sh stays outside release packages.
        for name in [".openomsi-files", "start.sh"] {
            if p.dir.join(name).is_file() {
                std::fs::copy(p.dir.join(name), backup.join(name))?;
            }
        }
        std::fs::write(backup.join("RECOVERY.txt"),format!("Program backup before updating {current} to {next}.\nClose openOMSI and the editor before restoring these files to {}.\nMaps and mods are outside this backup.\n",p.dir.display()))?;
        log::info!(
            "{} update: program backup {}",
            channel.title(),
            backup.display()
        );
        Ok(())
    })();
    if channel == Channel::Editor {
        let _ = std::fs::remove_file(zip_file);
    }
    result
}
fn copy_tree(from: &Path, to: &Path) -> anyhow::Result<()> {
    if from.symlink_metadata()?.file_type().is_symlink() {
        bail!("Cannot back up linked program files: {}", from.display());
    }
    if from.is_dir() {
        std::fs::create_dir_all(to)?;
        for entry in std::fs::read_dir(from)? {
            let entry = entry?;
            copy_tree(&entry.path(), &to.join(entry.file_name()))?;
        }
    } else {
        std::fs::copy(from, to)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let p = std::env::temp_dir().join(format!(
                "editor-channel-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&p).unwrap();
            Self(p)
        }
        fn place(&self, name: &str) -> Place {
            let dir = self.0.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("openomsi"), b"old-program").unwrap();
            Place {
                exe: dir.join("openomsi"),
                dir,
                bundle: None,
            }
        }
        fn archive(
            &self,
            version: &str,
            extra: Option<(&str, tar::EntryType)>,
        ) -> std::path::PathBuf {
            let p = self.0.join("update.tar.gz");
            let gzip = flate2::write::GzEncoder::new(
                std::fs::File::create(&p).unwrap(),
                flate2::Compression::fast(),
            );
            let mut tar = tar::Builder::new(gzip);
            let mut append = |name: &str, data: &[u8], kind: tar::EntryType| {
                let mut h = tar::Header::new_gnu();
                h.set_size(data.len() as u64);
                h.set_mode(0o755);
                h.set_entry_type(kind);
                if kind.is_symlink() {
                    h.set_link_name("/tmp/should-never-be-written").unwrap();
                }
                h.set_cksum();
                tar.append_data(&mut h, format!("openOMSI-Editor-{version}/{name}"), data)
                    .unwrap();
            };
            append("openomsi", b"#!/bin/sh\necho 'openomsi editor self-test ok'\n", tar::EntryType::Regular);
            append("openomsi-launcher-app", b"test-launcher", tar::EntryType::Regular);
            append(
                "VERSION-EDITOR.txt",
                format!("Editor {version}\nopenOMSI-Basis 0.2.27\n").as_bytes(),
                tar::EntryType::Regular,
            );
            append("docs/guide.pdf", b"new-guide", tar::EntryType::Regular);
            if let Some((name, kind)) = extra {
                append(name, b"", kind);
            }
            tar.into_inner().unwrap().finish().unwrap();
            p
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn release(version: &str, prerelease: bool) -> serde_json::Value {
        json!({"tag_name":format!("v{version}"),"prerelease":prerelease,"draft":false,"assets":[{"name":format!("openOMSI-Editor-{version}-linux-x64.tar.gz"),"browser_download_url":"https://github.com/test/archive.tar.gz","size":123,"digest":format!("sha256:{}","a".repeat(64))}]})
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn editor_prereleases_are_separate_and_never_downgrade_local_work() {
        let published = json!([release("0.7.22-pre", true), release("0.7.23-pre", true)]);
        assert!(editor_release(&published, "0.7.23-pre-local")
            .unwrap()
            .is_none());
        let future = json!([
            release("0.7.24-pre", true),
            release("0.7.25-pre", true),
            release("0.7.23-pre", true)
        ]);
        assert_eq!(
            editor_release(&future, "0.7.23-pre")
                .unwrap()
                .unwrap()
                .version,
            "0.7.25-pre"
        );
        let mut draft = release("8.0.0-pre", true);
        draft["draft"] = json!(true);
        assert!(editor_release(&json!([draft]), "0.7.23-pre")
            .unwrap()
            .is_none());
    }
    #[test]
    fn stable_release_supersedes_its_prerelease() {
        assert!(release_newer("1.0.0", "1.0.0-pre"));
        assert!(!release_newer("1.0.0-pre", "1.0.0"));
    }
    #[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
    #[test]
    fn unsupported_platform_cannot_offer_a_linux_editor_package() {
        let published = json!([release("0.8.0-pre", true)]);
        let error = editor_release(&published, "0.7.23-pre").unwrap_err();
        assert_eq!(error.to_string(), "No automatic editor update package for this platform.");
    }
    #[test]
    fn official_channel_uses_the_separate_game_version() {
        let v = json!({"tag_name":"v0.2.38","draft":false,"prerelease":false,"assets":[{"name":updater::asset_name("0.2.38").unwrap(),"browser_download_url":"https://github.com/openOMSI-org/openOMSI/releases/download/v0.2.38/openOMSI-0.2.38-linux-x64.zip","size":123,"digest":format!("sha256:{}","b".repeat(64))}]});
        assert_eq!(
            updater::parse_release(&v, "0.2.27", false)
                .unwrap()
                .unwrap()
                .version,
            "0.2.38"
        );
        assert!(updater::parse_release(&v, "0.2.38", false).unwrap().is_none());
    }
    #[test]
    fn the_game_channel_cannot_replace_the_editor() {
        let f = Fixture::new();
        let editor = f.place("editor");
        let game = f.place("game");
        std::fs::write(editor.dir.join("VERSION-EDITOR.txt"), "Editor 0.7.23-pre\n").unwrap();
        assert!(validate_target(Channel::Game, &editor, &editor.exe).is_err());
        assert!(validate_target(Channel::Game, &game, &editor.exe).is_ok());
        let editor_target = validate_target(Channel::Editor, &editor, &editor.exe);
        if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            assert!(editor_target.is_ok());
        } else {
            assert_eq!(editor_target.unwrap_err().to_string(),
                "Automatic editor updates require the Linux x64 editor package with VERSION-EDITOR.txt.");
        }
        assert!(validate_target(Channel::Editor, &game, &editor.exe).is_err());
        assert_eq!(
            installed_version(Channel::Editor, &editor.exe).unwrap(),
            "0.7.23-pre"
        );
    }
    #[cfg(unix)]
    #[test]
    fn editor_package_updates_program_and_manuals_and_keeps_maps_mods_and_game() {
        let f = Fixture::new();
        let editor = f.place("editor");
        let game = f.place("game");
        for name in ["maps/test.cfg", "Vehicles/bus.hof", "start.sh"] {
            let p = editor.dir.join(name);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, b"keep-user-file").unwrap();
        }
        let a = f.archive("0.7.24-pre", None);
        backup_program(&a, &editor, Channel::Editor, "0.7.23-pre", "0.7.24-pre").unwrap();
        install_editor_archive(&a, &editor, "0.7.24-pre").unwrap();
        assert_eq!(std::fs::read(&editor.exe).unwrap(), b"#!/bin/sh\necho 'openomsi editor self-test ok'\n");
        assert_eq!(
            std::fs::read(editor.dir.join("docs/guide.pdf")).unwrap(),
            b"new-guide"
        );
        assert_eq!(std::fs::read(&game.exe).unwrap(), b"old-program");
        for name in ["maps/test.cfg", "Vehicles/bus.hof", "start.sh"] {
            assert_eq!(
                std::fs::read(editor.dir.join(name)).unwrap(),
                b"keep-user-file"
            );
        }
        let backup = std::fs::read_dir(editor.dir.join(".editor-update-backups"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(
            std::fs::read(backup.join("openomsi")).unwrap(),
            b"old-program"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_ne!(
                std::fs::metadata(editor.exe).unwrap().permissions().mode() & 0o111,
                0
            );
        }
    }
    #[test]
    fn bad_editor_packages_change_nothing() {
        let f = Fixture::new();
        let editor = f.place("editor");
        let a = f.archive("0.7.24-pre", None);
        assert!(install_editor_archive(&a, &editor, "0.7.25-pre").is_err());
        for extra in [
            ("assets/link", tar::EntryType::Symlink),
            ("maps/global.cfg", tar::EntryType::Regular),
            ("LICENSE", tar::EntryType::Regular),
            ("README.md", tar::EntryType::Regular),
        ] {
            let a = f.archive("0.7.24-pre", Some(extra));
            assert!(install_editor_archive(&a, &editor, "0.7.24-pre").is_err());
            assert_eq!(std::fs::read(&editor.exe).unwrap(), b"old-program");
        }
    }
    /// Run against the exact release artifact, not a synthetic fixture.
    #[cfg(target_os = "linux")]
    #[test]
    #[ignore = "set EDITOR_RELEASE_ARCHIVE and EDITOR_RELEASE_VERSION to validate a built release"]
    fn built_release_installs_with_existing_updater() {
        let archive = std::path::PathBuf::from(std::env::var("EDITOR_RELEASE_ARCHIVE").unwrap());
        let version = std::env::var("EDITOR_RELEASE_VERSION").unwrap();
        let f = Fixture::new();
        let editor = f.place("editor");
        std::fs::create_dir_all(editor.dir.join("maps")).unwrap();
        std::fs::write(editor.dir.join("maps/user-map.cfg"), b"keep-user-map").unwrap();
        backup_program(&archive, &editor, Channel::Editor, "0.8.0-pre", &version).unwrap();
        install_editor_archive(&archive, &editor, &version).unwrap();
        assert_eq!(installed_version(Channel::Editor, &editor.exe).unwrap(), version);
        assert_eq!(std::fs::read(editor.dir.join("maps/user-map.cfg")).unwrap(), b"keep-user-map");
        for name in ["LICENSE", "README.md", "openOMSI-Editor-Kurzanleitung.pdf", "openOMSI-Editor-Quick-Guide-EN.pdf"] {
            assert!(editor.dir.join("docs").join(name).is_file(), "missing {name}");
        }
    }
    #[test]
    fn checksum_is_checked_again_before_replacing_files() {
        use sha2::Digest;
        let f = Fixture::new();
        let file = f.0.join("archive");
        std::fs::write(&file, b"verified-data").unwrap();
        let r = Release {
            version: "0.7.24-pre".into(),
            page: String::new(),
            notes: String::new(),
            asset_name: "archive".into(),
            asset_url: String::new(),
            size: 13,
            sha256: Some(sha2::Sha256::digest(b"verified-data").iter().map(|b|format!("{b:02x}")).collect()),
        };
        verify_checksum(&file, &r).unwrap();
        std::fs::write(&file, b"modified-data").unwrap();
        assert!(verify_checksum(&file, &r).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn official_package_updates_only_game_program_and_protects_content() {
        let f = Fixture::new();
        let game = f.place("game");
        let editor = f.place("editor");
        let a = f.0.join("official.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&a).unwrap());
        zip.start_file(
            "openomsi",
            zip::write::SimpleFileOptions::default().unix_permissions(0o755),
        )
        .unwrap();
        zip.write_all(b"#!/bin/sh\necho 'openomsi game self-test ok'\n").unwrap();
        zip.finish().unwrap();
        std::fs::create_dir_all(game.dir.join("Vehicles")).unwrap();
        std::fs::write(game.dir.join("Vehicles/my-bus.hof"), b"custom-hof").unwrap();
        backup_program(&a, &game, Channel::Game, "0.2.27", "0.2.38").unwrap();
        updater::install_archive(&a, &game).unwrap();
        assert_eq!(std::fs::read(&game.exe).unwrap(), b"#!/bin/sh\necho 'openomsi game self-test ok'\n");
        assert_eq!(std::fs::read(&editor.exe).unwrap(), b"old-program");
        assert_eq!(
            std::fs::read(game.dir.join("Vehicles/my-bus.hof")).unwrap(),
            b"custom-hof"
        );
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&a).unwrap());
        zip.start_file("openomsi", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"game").unwrap();
        zip.start_file(
            "Vehicles/my-bus.hof",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(b"replace-hof").unwrap();
        zip.finish().unwrap();
        assert!(backup_program(&a, &game, Channel::Game, "0.2.38", "0.2.39").is_err());
        assert_eq!(
            std::fs::read(game.dir.join("Vehicles/my-bus.hof")).unwrap(),
            b"custom-hof"
        );
    }
}

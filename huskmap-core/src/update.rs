//! Newer huskmap from GitHub Releases, installed the way this one was installed.
//!
//! The only network huskmap ever does. Checked at most once a day, off with
//! `HUSKMAP_NO_UPDATE_CHECK=1` or `check_updates: false` in settings. Every download is
//! verified against the release's `SHA256SUMS` before anything runs.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::Error;

pub const REPO: &str = "LucasCavalheri/huskmap";
pub const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const LATEST_URL: &str = "https://api.github.com/repos/LucasCavalheri/huskmap/releases/latest";
pub const SUMS_ASSET: &str = "SHA256SUMS";
/// `Fetch::text` fails with this when the URL does not exist (no release published yet).
pub const NOT_FOUND: &str = "not found";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: String,
    pub page: String,
    pub notes: String,
    pub assets: Vec<Asset>,
}

impl Release {
    pub fn asset(&self, name: &str) -> Option<&Asset> {
        self.assets.iter().find(|a| a.name == name)
    }

    /// First few meaningful lines of the notes, for a modal.
    pub fn summary(&self, lines: usize) -> Vec<String> {
        self.notes
            .lines()
            .map(|l| l.trim().trim_start_matches(['-', '*', '#']).trim())
            .filter(|l| !l.is_empty())
            .take(lines)
            .map(str::to_string)
            .collect()
    }
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

#[derive(Deserialize)]
struct GhRelease {
    tag_name: String,
    #[serde(default)]
    html_url: String,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    assets: Vec<GhAsset>,
}

pub fn normalize_tag(tag: &str) -> String {
    tag.trim().trim_start_matches(['v', 'V']).to_string()
}

pub fn parse_release(json: &str) -> Result<Release, Error> {
    let gh: GhRelease = serde_json::from_str(json)?;
    if gh.draft {
        return Err(Error::update("latest release is a draft"));
    }
    Ok(Release {
        version: normalize_tag(&gh.tag_name),
        page: gh.html_url,
        notes: gh.body.unwrap_or_default(),
        assets: gh
            .assets
            .into_iter()
            .map(|a| Asset {
                name: a.name,
                url: a.browser_download_url,
                size: a.size,
            })
            .collect(),
    })
}

/// SemVer ordering on `major.minor.patch[-pre]`; a pre-release sorts below its release.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    fn split(v: &str) -> (Vec<u64>, Option<String>) {
        let v = normalize_tag(v);
        let v = v.split('+').next().unwrap_or("").to_string();
        let (core, pre) = match v.split_once('-') {
            Some((c, p)) => (c.to_string(), Some(p.to_string())),
            None => (v, None),
        };
        let nums = core
            .split('.')
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .chain(std::iter::repeat(0))
            .take(3)
            .collect();
        (nums, pre)
    }
    let (an, ap) = split(a);
    let (bn, bp) = split(b);
    an.cmp(&bn).then_with(|| match (ap, bp) {
        (None, None) => Ordering::Equal,
        (None, Some(_)) => Ordering::Greater,
        (Some(_), None) => Ordering::Less,
        (Some(x), Some(y)) => x.cmp(&y),
    })
}

pub fn is_newer(candidate: &str, current: &str) -> bool {
    compare_versions(candidate, current) == Ordering::Greater
}

/// How this binary got onto the machine, which decides how it updates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallKind {
    Deb,
    Rpm,
    Pacman,
    Apk,
    /// The portable tarball, installed by its `install.sh` (writable or not).
    Portable(PathBuf),
    /// `cargo install`, a dev build, anything we must not overwrite.
    Unmanaged(PathBuf),
}

impl InstallKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Deb => "deb",
            Self::Rpm => "rpm",
            Self::Pacman => "pacman",
            Self::Apk => "apk",
            Self::Portable(_) => "portable",
            Self::Unmanaged(_) => "unmanaged",
        }
    }
}

/// Everything update needs from the machine. Tests use a fake.
pub trait Host {
    fn exe(&self) -> Option<PathBuf>;
    /// Does `program args…` exit 0? Used for package ownership queries.
    fn succeeds(&self, program: &str, args: &[&str]) -> bool;
    /// Run a command, failing with its message.
    fn run(&self, program: &str, args: &[&str]) -> Result<(), Error>;
    /// `x86_64` or `aarch64`.
    fn arch(&self) -> &'static str;
    fn has(&self, program: &str) -> bool;
}

pub trait Fetch {
    fn text(&self, url: &str) -> Result<String, Error>;
    fn file(&self, url: &str, dest: &Path) -> Result<(), Error>;
}

pub fn detect_install_kind(host: &dyn Host) -> InstallKind {
    let Some(exe) = host.exe() else {
        return InstallKind::Unmanaged(PathBuf::from("huskmap"));
    };
    let path = exe.to_string_lossy().to_string();
    if path.starts_with("/usr/") && !path.starts_with("/usr/local/") {
        let owners: [(&str, &[&str], InstallKind); 4] = [
            ("dpkg", &["-S"], InstallKind::Deb),
            ("rpm", &["-qf"], InstallKind::Rpm),
            ("pacman", &["-Qo"], InstallKind::Pacman),
            ("apk", &["info", "--who-owns"], InstallKind::Apk),
        ];
        for (tool, args, kind) in owners {
            let mut full: Vec<&str> = args.to_vec();
            full.push(&path);
            if host.has(tool) && host.succeeds(tool, &full) {
                return kind;
            }
        }
    }
    let managed_elsewhere = path.contains("/target/") || path.contains("/.cargo/bin/");
    if managed_elsewhere {
        InstallKind::Unmanaged(exe)
    } else {
        InstallKind::Portable(exe)
    }
}

/// Asset name for this install, matching `packaging/build-linux-packages.sh`.
pub fn asset_name(kind: &InstallKind, arch: &str, version: &str) -> Option<String> {
    let deb = if arch == "aarch64" { "arm64" } else { "amd64" };
    let tar = if arch == "aarch64" { "arm64" } else { "x64" };
    Some(match kind {
        InstallKind::Deb => format!("huskmap_{version}_{deb}.deb"),
        InstallKind::Rpm => format!("huskmap-{version}-1.{arch}.rpm"),
        InstallKind::Pacman => format!("huskmap-{version}-1-{arch}.pkg.tar.zst"),
        InstallKind::Apk => format!("huskmap-{version}-r0-{arch}.apk"),
        InstallKind::Portable(_) => format!("huskmap-linux-{tar}.tar.gz"),
        InstallKind::Unmanaged(_) => return None,
    })
}

/// `<hex>  <name>` lines, as `sha256sum` writes them.
pub fn parse_sums(text: &str, name: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let mut parts = line.split_whitespace();
        let hash = parts.next()?;
        let file = parts.next()?.trim_start_matches('*');
        (file == name && hash.len() == 64 && hash.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| hash.to_ascii_lowercase())
    })
}

pub fn sha256_file(path: &Path) -> Result<String, Error> {
    let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
    let digest = Sha256::digest(&bytes);
    Ok(digest.iter().map(|b| format!("{b:02x}")).collect())
}

/// `None` when this build is current (or newer).
pub fn check(fetch: &dyn Fetch, current: &str) -> Result<Option<Release>, Error> {
    let json = match fetch.text(LATEST_URL) {
        Err(Error::Update(msg)) if msg == NOT_FOUND => return Ok(None),
        other => other?,
    };
    let release = parse_release(&json)?;
    Ok(is_newer(&release.version, current).then_some(release))
}

/// Download, verify, install. Returns the binary to relaunch.
pub fn install(
    release: &Release,
    kind: &InstallKind,
    fetch: &dyn Fetch,
    host: &dyn Host,
    workdir: &Path,
) -> Result<PathBuf, Error> {
    let name = asset_name(kind, host.arch(), &release.version)
        .ok_or_else(|| Error::update(crate::copy::get().update_unmanaged))?;
    let asset = release
        .asset(&name)
        .ok_or_else(|| Error::update(format!("release {} has no {name}", release.version)))?;
    let sums_asset = release
        .asset(SUMS_ASSET)
        .ok_or_else(|| Error::update("release has no SHA256SUMS; refusing"))?;
    let expected = parse_sums(&fetch.text(&sums_asset.url)?, &name)
        .ok_or_else(|| Error::update(format!("SHA256SUMS does not list {name}; refusing")))?;
    std::fs::create_dir_all(workdir).map_err(|e| Error::io(workdir, e))?;
    let file = workdir.join(&name);
    fetch.file(&asset.url, &file)?;
    if sha256_file(&file)? != expected {
        let _ = std::fs::remove_file(&file);
        return Err(Error::update(crate::copy::get().update_checksum));
    }
    let f = file.to_string_lossy().to_string();
    let elevate = |script: String| -> Result<(), Error> {
        if !host.has("pkexec") {
            return Err(Error::update(crate::copy::get().update_no_pkexec));
        }
        host.run("pkexec", &["sh", "-c", &script])
    };
    let q = shell_quote(&f);
    match kind {
        InstallKind::Deb => elevate(format!(
            "if command -v apt-get >/dev/null; then DEBIAN_FRONTEND=noninteractive apt-get install -y {q}; else dpkg -i {q}; fi"
        ))?,
        InstallKind::Rpm => elevate(format!(
            "if command -v dnf >/dev/null; then dnf install -y {q}; elif command -v zypper >/dev/null; then zypper --non-interactive install --allow-unsigned-rpm {q}; else rpm -Uvh {q}; fi"
        ))?,
        InstallKind::Pacman => elevate(format!("pacman -U --noconfirm {q}"))?,
        InstallKind::Apk => elevate(format!("apk add --allow-untrusted {q}"))?,
        InstallKind::Portable(exe) => {
            let unpack = workdir.join("unpack");
            std::fs::create_dir_all(&unpack).map_err(|e| Error::io(&unpack, e))?;
            host.run("tar", &["-xzf", &f, "-C", &unpack.to_string_lossy()])?;
            let fresh = unpack.join("huskmap").join("bin").join("huskmap");
            if !fresh.is_file() {
                return Err(Error::update("archive has no huskmap/bin/huskmap"));
            }
            replace_binary(&fresh, exe)?;
            return Ok(exe.clone());
        }
        InstallKind::Unmanaged(_) => unreachable!("asset_name returned None"),
    }
    Ok(PathBuf::from("/usr/bin/huskmap"))
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Swap the running binary in place: copy next to it, then rename over it. Linux keeps the
/// old inode alive for the running process.
pub fn replace_binary(fresh: &Path, exe: &Path) -> Result<(), Error> {
    use std::os::unix::fs::PermissionsExt;
    let staged = exe.with_extension("new");
    std::fs::copy(fresh, &staged).map_err(|e| Error::io(&staged, e))?;
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))
        .map_err(|e| Error::io(&staged, e))?;
    std::fs::rename(&staged, exe).map_err(|e| Error::io(exe, e))
}

/// Real machine.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemHost;

impl Host for SystemHost {
    fn exe(&self) -> Option<PathBuf> {
        std::env::current_exe().ok()
    }

    fn succeeds(&self, program: &str, args: &[&str]) -> bool {
        std::process::Command::new(program)
            .args(args)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    fn run(&self, program: &str, args: &[&str]) -> Result<(), Error> {
        let out = std::process::Command::new(program)
            .args(args)
            .output()
            .map_err(|e| Error::update(format!("{program}: {e}")))?;
        if out.status.success() {
            return Ok(());
        }
        // pkexec: 126 = dismissed, 127 = not authorized.
        if program == "pkexec" && matches!(out.status.code(), Some(126 | 127)) {
            return Err(Error::update(crate::copy::get().update_declined));
        }
        let msg = String::from_utf8_lossy(&out.stderr);
        Err(Error::update(format!("{program}: {}", msg.trim())))
    }

    fn arch(&self) -> &'static str {
        std::env::consts::ARCH
    }

    fn has(&self, program: &str) -> bool {
        std::env::var_os("PATH").is_some_and(|paths| {
            std::env::split_paths(&paths).any(|dir| dir.join(program).is_file())
        })
    }
}

/// HTTPS through `ureq` (rustls).
#[derive(Debug, Default, Clone, Copy)]
pub struct Https;

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent(&format!("huskmap/{CURRENT_VERSION}"))
        .build()
}

impl Fetch for Https {
    fn text(&self, url: &str) -> Result<String, Error> {
        agent()
            .get(url)
            .set("Accept", "application/vnd.github+json")
            .call()
            .map_err(|e| match e {
                ureq::Error::Status(404, _) => Error::update(NOT_FOUND),
                e => Error::update(e.to_string()),
            })?
            .into_string()
            .map_err(|e| Error::update(e.to_string()))
    }

    fn file(&self, url: &str, dest: &Path) -> Result<(), Error> {
        let resp = agent()
            .get(url)
            .call()
            .map_err(|e| Error::update(e.to_string()))?;
        let mut out = std::fs::File::create(dest).map_err(|e| Error::io(dest, e))?;
        std::io::copy(&mut resp.into_reader(), &mut out).map_err(|e| Error::io(dest, e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    const JSON: &str = r###"{
        "tag_name": "v0.2.0",
        "html_url": "https://github.com/LucasCavalheri/huskmap/releases/tag/v0.2.0",
        "body": "## Highlights\n\n- Forced sends from the map\n- Faster scans\n\n* Grok sessions\n",
        "draft": false,
        "assets": [
            {"name": "huskmap_0.2.0_amd64.deb", "browser_download_url": "https://x/deb", "size": 3},
            {"name": "huskmap-linux-x64.tar.gz", "browser_download_url": "https://x/tar", "size": 3},
            {"name": "SHA256SUMS", "browser_download_url": "https://x/sums", "size": 1}
        ]
    }"###;

    #[derive(Default)]
    struct FakeFetch {
        texts: HashMap<String, String>,
        files: HashMap<String, Vec<u8>>,
    }

    impl Fetch for FakeFetch {
        fn text(&self, url: &str) -> Result<String, Error> {
            self.texts
                .get(url)
                .cloned()
                .ok_or_else(|| Error::update("offline"))
        }
        fn file(&self, url: &str, dest: &Path) -> Result<(), Error> {
            let body = self.files.get(url).ok_or_else(|| Error::update("404"))?;
            std::fs::write(dest, body).map_err(|e| Error::io(dest, e))
        }
    }

    struct FakeHost {
        exe: Option<PathBuf>,
        owner: Option<&'static str>,
        tools: Vec<&'static str>,
        ran: RefCell<Vec<String>>,
        fail_run: bool,
    }

    impl FakeHost {
        fn at(exe: &str, owner: Option<&'static str>) -> Self {
            Self {
                exe: Some(PathBuf::from(exe)),
                owner,
                tools: vec!["dpkg", "rpm", "pacman", "apk", "pkexec", "tar"],
                ran: RefCell::new(vec![]),
                fail_run: false,
            }
        }
    }

    impl Host for FakeHost {
        fn exe(&self) -> Option<PathBuf> {
            self.exe.clone()
        }
        fn succeeds(&self, program: &str, _: &[&str]) -> bool {
            self.owner == Some(program)
        }
        fn run(&self, program: &str, args: &[&str]) -> Result<(), Error> {
            self.ran
                .borrow_mut()
                .push(format!("{program} {}", args.join(" ")));
            if self.fail_run {
                return Err(Error::update("denied"));
            }
            if program == "tar" {
                // emulate unpacking the portable archive
                let dest = PathBuf::from(args[3]);
                std::fs::create_dir_all(dest.join("huskmap/bin")).unwrap();
                std::fs::write(dest.join("huskmap/bin/huskmap"), "new binary").unwrap();
            }
            Ok(())
        }
        fn arch(&self) -> &'static str {
            "x86_64"
        }
        fn has(&self, program: &str) -> bool {
            self.tools.contains(&program)
        }
    }

    fn sha(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    fn fetch_with(deb: &[u8], sums_for: &[u8]) -> FakeFetch {
        let mut f = FakeFetch::default();
        f.texts.insert(LATEST_URL.into(), JSON.into());
        f.texts.insert(
            "https://x/sums".into(),
            format!(
                "{}  huskmap_0.2.0_amd64.deb\n{}  huskmap-linux-x64.tar.gz\n",
                sha(sums_for),
                sha(b"tar")
            ),
        );
        f.files.insert("https://x/deb".into(), deb.to_vec());
        f.files.insert("https://x/tar".into(), b"tar".to_vec());
        f
    }

    #[test]
    fn versions() {
        assert_eq!(compare_versions("0.2.0", "0.1.9"), Ordering::Greater);
        assert_eq!(compare_versions("v1.0.0", "1.0"), Ordering::Equal);
        assert_eq!(compare_versions("1.0.0-rc.1", "1.0.0"), Ordering::Less);
        assert_eq!(
            compare_versions("1.0.0-rc.2", "1.0.0-rc.1"),
            Ordering::Greater
        );
        assert_eq!(compare_versions("1.0.0", "1.0.0-rc.1"), Ordering::Greater);
        assert_eq!(compare_versions("1.0.0+build", "1.0.0"), Ordering::Equal);
        assert!(is_newer("0.10.0", "0.9.9"), "numeric, not lexical");
        assert!(!is_newer(CURRENT_VERSION, CURRENT_VERSION));
        assert_eq!(normalize_tag(" v0.3.1 "), "0.3.1");
    }

    #[test]
    fn parses_release_and_notes() {
        let r = parse_release(JSON).unwrap();
        assert_eq!(r.version, "0.2.0");
        assert_eq!(r.assets.len(), 3);
        assert_eq!(
            r.summary(2),
            vec!["Highlights", "Forced sends from the map"]
        );
        assert!(r.asset("SHA256SUMS").is_some());
        assert!(parse_release(r#"{"tag_name":"v1","draft":true}"#).is_err());
        assert!(parse_release("nope").is_err());
    }

    #[test]
    fn install_kind_detection() {
        let cases = [
            ("/usr/bin/huskmap", Some("dpkg"), InstallKind::Deb),
            ("/usr/bin/huskmap", Some("rpm"), InstallKind::Rpm),
            ("/usr/bin/huskmap", Some("pacman"), InstallKind::Pacman),
            ("/usr/bin/huskmap", Some("apk"), InstallKind::Apk),
            (
                "/usr/local/bin/huskmap",
                Some("dpkg"),
                InstallKind::Portable(PathBuf::from("/usr/local/bin/huskmap")),
            ),
            (
                "/home/me/.cargo/bin/huskmap",
                None,
                InstallKind::Unmanaged(PathBuf::from("/home/me/.cargo/bin/huskmap")),
            ),
            (
                "/home/me/dev/huskmap/target/release/huskmap",
                None,
                InstallKind::Unmanaged(PathBuf::from(
                    "/home/me/dev/huskmap/target/release/huskmap",
                )),
            ),
        ];
        for (exe, owner, want) in cases {
            assert_eq!(
                detect_install_kind(&FakeHost::at(exe, owner)),
                want,
                "{exe} {owner:?}"
            );
        }
        let mut none = FakeHost::at("/x", None);
        none.exe = None;
        assert!(matches!(
            detect_install_kind(&none),
            InstallKind::Unmanaged(_)
        ));
        for k in [
            InstallKind::Deb,
            InstallKind::Rpm,
            InstallKind::Pacman,
            InstallKind::Apk,
        ] {
            assert!(!k.label().is_empty());
        }
        assert_eq!(InstallKind::Portable(PathBuf::new()).label(), "portable");
        assert_eq!(InstallKind::Unmanaged(PathBuf::new()).label(), "unmanaged");
    }

    #[test]
    fn asset_names_match_packaging() {
        let v = "0.2.0";
        assert_eq!(
            asset_name(&InstallKind::Deb, "x86_64", v).unwrap(),
            "huskmap_0.2.0_amd64.deb"
        );
        assert_eq!(
            asset_name(&InstallKind::Deb, "aarch64", v).unwrap(),
            "huskmap_0.2.0_arm64.deb"
        );
        assert_eq!(
            asset_name(&InstallKind::Rpm, "aarch64", v).unwrap(),
            "huskmap-0.2.0-1.aarch64.rpm"
        );
        assert_eq!(
            asset_name(&InstallKind::Pacman, "x86_64", v).unwrap(),
            "huskmap-0.2.0-1-x86_64.pkg.tar.zst"
        );
        assert_eq!(
            asset_name(&InstallKind::Apk, "x86_64", v).unwrap(),
            "huskmap-0.2.0-r0-x86_64.apk"
        );
        assert_eq!(
            asset_name(&InstallKind::Portable(PathBuf::new()), "aarch64", v).unwrap(),
            "huskmap-linux-arm64.tar.gz"
        );
        assert!(asset_name(&InstallKind::Unmanaged(PathBuf::new()), "x86_64", v).is_none());
    }

    #[test]
    fn sums_parsing() {
        let h = "a".repeat(64);
        let text = format!("{h}  huskmap.deb\n{h} *other\nshort  x\n");
        assert_eq!(parse_sums(&text, "huskmap.deb"), Some(h.clone()));
        assert_eq!(parse_sums(&text, "other"), Some(h));
        assert_eq!(parse_sums(&text, "x"), None);
        assert_eq!(parse_sums(&text, "missing"), None);
    }

    #[test]
    fn check_says_newer_or_nothing() {
        let f = fetch_with(b"deb", b"deb");
        assert_eq!(check(&f, "0.1.0").unwrap().unwrap().version, "0.2.0");
        assert!(check(&f, "0.2.0").unwrap().is_none());
        assert!(check(&FakeFetch::default(), "0.1.0").is_err());
        struct Missing;
        impl Fetch for Missing {
            fn text(&self, _: &str) -> Result<String, Error> {
                Err(Error::update(NOT_FOUND))
            }
            fn file(&self, _: &str, _: &Path) -> Result<(), Error> {
                Ok(())
            }
        }
        assert!(
            check(&Missing, "0.1.0").unwrap().is_none(),
            "no release yet is not an error"
        );
        assert!(Missing.file("x", Path::new("/x")).is_ok());
    }

    #[test]
    fn install_verifies_then_elevates() {
        crate::copy::with_locale(crate::copy::Locale::En, || {
            let tmp = tempfile::tempdir().unwrap();
            let release = parse_release(JSON).unwrap();
            let host = FakeHost::at("/usr/bin/huskmap", Some("dpkg"));
            let ok = fetch_with(b"deb", b"deb");
            let exe = install(&release, &InstallKind::Deb, &ok, &host, tmp.path()).unwrap();
            assert_eq!(exe, PathBuf::from("/usr/bin/huskmap"));
            let ran = host.ran.borrow().clone();
            assert!(
                ran[0].starts_with("pkexec sh -c if command -v apt-get"),
                "{ran:?}"
            );

            // a tampered download never reaches pkexec
            let host = FakeHost::at("/usr/bin/huskmap", Some("dpkg"));
            let bad = fetch_with(b"evil", b"deb");
            let err = install(&release, &InstallKind::Deb, &bad, &host, tmp.path()).unwrap_err();
            assert!(err.to_string().contains("SHA256SUMS"));
            assert!(host.ran.borrow().is_empty());

            // no pkexec, no install
            let mut host = FakeHost::at("/usr/bin/huskmap", Some("dpkg"));
            host.tools.retain(|t| *t != "pkexec");
            let err = install(&release, &InstallKind::Deb, &ok, &host, tmp.path()).unwrap_err();
            assert!(err.to_string().contains("pkexec"));

            // missing asset, missing sums, unlisted asset, unmanaged
            assert!(
                install(
                    &release,
                    &InstallKind::Rpm,
                    &ok,
                    &FakeHost::at("/usr/bin/huskmap", None),
                    tmp.path()
                )
                .is_err()
            );
            let mut nosums = release.clone();
            nosums.assets.retain(|a| a.name != SUMS_ASSET);
            assert!(
                install(
                    &nosums,
                    &InstallKind::Deb,
                    &ok,
                    &FakeHost::at("/usr/bin/huskmap", None),
                    tmp.path()
                )
                .is_err()
            );
            let mut unlisted = fetch_with(b"deb", b"deb");
            unlisted
                .texts
                .insert("https://x/sums".into(), String::new());
            assert!(
                install(
                    &release,
                    &InstallKind::Deb,
                    &unlisted,
                    &FakeHost::at("/usr/bin/huskmap", None),
                    tmp.path()
                )
                .is_err()
            );
            let err = install(
                &release,
                &InstallKind::Unmanaged(PathBuf::from("/c")),
                &ok,
                &FakeHost::at("/c", None),
                tmp.path(),
            )
            .unwrap_err();
            assert!(err.to_string().contains("cargo"));
        });
    }

    #[test]
    fn every_package_manager_command() {
        let tmp = tempfile::tempdir().unwrap();
        let mut release = parse_release(JSON).unwrap();
        let mut f = fetch_with(b"deb", b"deb");
        let mut sums = String::new();
        for (kind, name) in [
            (InstallKind::Rpm, "huskmap-0.2.0-1.x86_64.rpm"),
            (InstallKind::Pacman, "huskmap-0.2.0-1-x86_64.pkg.tar.zst"),
            (InstallKind::Apk, "huskmap-0.2.0-r0-x86_64.apk"),
        ] {
            release.assets.push(Asset {
                name: name.into(),
                url: format!("https://x/{name}"),
                size: 1,
            });
            f.files
                .insert(format!("https://x/{name}"), name.as_bytes().to_vec());
            sums.push_str(&format!("{}  {name}\n", sha(name.as_bytes())));
            f.texts.insert("https://x/sums".into(), sums.clone());
            let host = FakeHost::at("/usr/bin/huskmap", None);
            install(&release, &kind, &f, &host, tmp.path()).unwrap();
            let ran = host.ran.borrow()[0].clone();
            let want = match kind {
                InstallKind::Rpm => "dnf install",
                InstallKind::Pacman => "pacman -U",
                _ => "apk add",
            };
            assert!(ran.contains(want), "{ran}");
        }
        let mut failing = FakeHost::at("/usr/bin/huskmap", None);
        failing.fail_run = true;
        assert!(install(&release, &InstallKind::Apk, &f, &failing, tmp.path()).is_err());
    }

    #[test]
    fn portable_swaps_the_binary_in_place() {
        let tmp = tempfile::tempdir().unwrap();
        let bin = tmp.path().join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let exe = bin.join("huskmap");
        std::fs::write(&exe, "old").unwrap();
        let release = parse_release(JSON).unwrap();
        let f = fetch_with(b"deb", b"deb");
        let host = FakeHost::at(&exe.to_string_lossy(), None);
        let kind = InstallKind::Portable(exe.clone());
        let back = install(&release, &kind, &f, &host, &tmp.path().join("work")).unwrap();
        assert_eq!(back, exe);
        assert_eq!(std::fs::read_to_string(&exe).unwrap(), "new binary");
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&exe).unwrap().permissions().mode() & 0o777,
            0o755
        );
        // an archive without the binary is refused
        struct EmptyTar;
        impl Host for EmptyTar {
            fn exe(&self) -> Option<PathBuf> {
                None
            }
            fn succeeds(&self, _: &str, _: &[&str]) -> bool {
                false
            }
            fn run(&self, _: &str, _: &[&str]) -> Result<(), Error> {
                Ok(())
            }
            fn arch(&self) -> &'static str {
                "x86_64"
            }
            fn has(&self, _: &str) -> bool {
                true
            }
        }
        let err = install(&release, &kind, &f, &EmptyTar, &tmp.path().join("work2")).unwrap_err();
        assert!(err.to_string().contains("archive"));
        assert!(EmptyTar.exe().is_none() && !EmptyTar.succeeds("x", &[]));
    }

    #[test]
    fn real_host_basics() {
        let h = SystemHost;
        assert!(h.exe().is_some());
        assert!(h.has("sh"));
        assert!(!h.has("definitely-not-a-program-huskmap"));
        assert!(h.succeeds("true", &[]));
        assert!(!h.succeeds("false", &[]));
        assert!(h.run("true", &[]).is_ok());
        assert!(h.run("false", &[]).is_err());
        assert!(h.run("definitely-not-a-program-huskmap", &[]).is_err());
        assert!(["x86_64", "aarch64"].contains(&h.arch()) || !h.arch().is_empty());
        assert_eq!(shell_quote("a'b"), "'a'\\''b'");
        let tmp = tempfile::tempdir().unwrap();
        assert!(sha256_file(&tmp.path().join("missing")).is_err());
        std::fs::write(tmp.path().join("f"), "abc").unwrap();
        assert_eq!(
            sha256_file(&tmp.path().join("f")).unwrap(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(replace_binary(&tmp.path().join("missing"), &tmp.path().join("x")).is_err());
    }
}

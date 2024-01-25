use crossterm::execute;
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use pep440_rs::{parse_version_specifiers, Version};
use regex::Regex;
use reqwest::Client;
use reqwest::Error;
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::fs::File;
use std::io::prelude::*;
use std::str::FromStr;
use std::{env, fs, io};
use thiserror::Error;
use tokio::runtime::Runtime;

static PYTHON_REGEX: &str = r"Python (\d+\.\d+\.\d+)";
static MCDR_REGEX: &str = r"Version: (\d+\.\d+\.?\d+?)";
static MODULE_REGEX: &str = r"^([0-9A-Za-z\.\*~=><\[\]]+ *)+$";
static MCDR_URL: &str = "https://mirrors.bfsu.edu.cn/pypi/web/json/mcdreforged";

pub fn python_url(version: Option<&str>) -> String {
    if let Some(v) = version {
        return format!(
            "https://registry.npmmirror.com/-/binary/python/{}/python-{}-amd64.exe",
            v, v
        );
    }
    return format!("https://registry.npmmirror.com/-/binary/python/");
}

pub fn cprintln(color: Color, s: &str) {
    execute!(
        std::io::stdout(),
        SetForegroundColor(color),
        Print(s),
        ResetColor
    )
    .unwrap();
}

pub fn pause() {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();
    write!(stdout, "Press Enter to continue...").unwrap();
    stdout.flush().unwrap();
    let _ = stdin.read(&mut [0u8]).unwrap();
}

pub fn panic_pause() {
    cprintln(
        Color::Red,
        "___________
* WARNING *
* This is NOT a issue of MCDReforged.
* DO NOT report this to MCDReforged.
___________
* MCDReforged discussion: @QQ(1101314858)
* MCDReee discussion: https://github.com/alex3236/MCDReee/discussions
* MCDReee issue tracker: https://github.com/alex3236/MCDReee/issues

",
    );
    pause();
}

#[derive(Deserialize, Serialize, Debug)]
struct PyPIData {
    info: MCDRMetadata,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct MCDRMetadata {
    pub requires_python: String,
    pub version: String,
}

pub fn get_mcdr_data() -> Result<MCDRMetadata, Error> {
    match reqwest::blocking::get(MCDR_URL) {
        Ok(r) => match r.json::<PyPIData>() {
            Ok(j) => return Ok(j.info),
            Err(e) => return Err(e),
        },
        Err(e) => return Err(e),
    };
}

#[derive(Debug)]
pub enum PythonVersion {
    Outdated,
    NotFound,
}

pub fn check_python(mcdr_meta: &MCDRMetadata) -> Result<String, PythonVersion> {
    let output = std::process::Command::new("python")
        .args(["--version"])
        .output();
    if let Ok(output) = output {
        let re = Regex::new(PYTHON_REGEX).unwrap();
        let out = String::from_utf8_lossy(&output.stdout);
        if let Some(captures) = re.captures(&out) {
            let python_version = captures.get(1).unwrap().as_str().to_string();
            let python_pep440 = Version::from_str(&python_version).unwrap();
            let accept = parse_version_specifiers(&mcdr_meta.requires_python)
                .unwrap()
                .iter()
                .all(|spec| spec.contains(&python_pep440));
            if accept {
                return Ok(python_version);
            }
            return Err(PythonVersion::Outdated);
        }
        return Err(PythonVersion::NotFound);
    } else {
        return Err(PythonVersion::NotFound);
    }
}

#[derive(Debug)]
pub enum MCDRResult {
    NoPip,
    NoMCDR,
    Outdated,
}

pub fn check_module(mcdr_meta: &MCDRMetadata) -> Result<String, MCDRResult> {
    let output = std::process::Command::new("pip")
        .args(["show", "mcdreforged"])
        .output();
    if let Ok(output) = output {
        if output.status.code().unwrap_or_default() != 0 {
            return Err(MCDRResult::NoMCDR);
        }
        let re = Regex::new(MCDR_REGEX).unwrap();
        let out = String::from_utf8_lossy(&output.stdout);
        if let Some(captures) = re.captures(&out) {
            let version = captures.get(1).unwrap().as_str().to_string();
            if Version::from_str(&version).unwrap() < Version::from_str(&mcdr_meta.version).unwrap()
            {
                return Err(MCDRResult::Outdated);
            }
            return Ok(version);
        }
        return Err(MCDRResult::NoMCDR);
    } else {
        Err(MCDRResult::NoPip)
    }
}

pub fn check_initialized() -> bool {
    // Check if MCDReforged initialized before
    // by checking folder struct
    fs::metadata("permission.yml").is_ok() && fs::metadata("config.yml").is_ok()
}

pub fn check_empty_folder() -> bool {
    // check is there any other file in current folder
    let self_path = env::current_exe().unwrap();
    let self_name = self_path.file_name().unwrap_or_default();
    fs::read_dir(".")
        .unwrap()
        .filter_map(|entry| entry.ok())
        .any(|entry| entry.path().file_name().unwrap_or_default() != self_name)
}

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("{0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("{0}")]
    IO(#[from] std::io::Error),
}

pub fn download_file(url: &str, path: &str) -> Result<(), DownloadError> {
    // executor::block_on(_download_file(url, path))
    Runtime::new().unwrap().block_on(_download_file(url, path))
}

pub async fn _download_file(url: &str, path: &str) -> Result<(), DownloadError> {
    // https://gist.github.com/giuliano-oliveira/4d11d6b3bb003dba3a1b53f43d81b30d

    // Reqwest setup
    let client = Client::new();
    let res = client.get(url).send().await?;
    let total_size = res.content_length().unwrap();

    // Indicatif setup
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::default_bar()
        .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .unwrap()
        .progress_chars("#>-"));
    pb.set_message(format!("Downloading {}", path));

    // download chunks
    let mut file = File::create(path)?;
    let mut downloaded: u64 = 0;
    let mut stream = res.bytes_stream();

    while let Some(item) = stream.next().await {
        let chunk = item?;
        file.write_all(&chunk)?;
        let new = min(downloaded + (chunk.len() as u64), total_size);
        downloaded = new;
        pb.set_position(new);
    }

    pb.finish_with_message(format!("Downloaded {}", path));
    return Ok(());
}

pub fn validate_modules(modules: &str) -> bool {
    Regex::new(MODULE_REGEX).unwrap().is_match(modules)
}

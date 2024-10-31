use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct Project {
    pub name: String,
    pub versions: Vec<DownloadableVersion>
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DownloadableVersion {
    version: VersionInformation,
    #[serde(rename = "location")] acquisition: AcquisitionMethod
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct VersionInformation {
    major: u32,
    minor: u32,
    #[serde(rename = "micro")] patch: Option<u32>,
    modifiers: Option<String>
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", content = "options")]
pub enum AcquisitionMethod {
    #[serde(rename = "github-repo-tag")]
    GitHubTag {
        #[serde(rename = "clone-url-http")] clone_url: String,
        tag: String
    },
    #[serde(rename = "jar-archive-link")]
    JarArchiveLink {
        url: String,
        verification: Vec<ArchiveVerificationMethod>
    },
    #[serde(rename = "tar-gz-archive-link")]
    TagGzArchiveLink {
        url: String,
        verification: Vec<ArchiveVerificationMethod>
    },
    #[serde(rename = "not-available")]
    NotAvailable{}
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type")]
pub enum ArchiveVerificationMethod {
    #[serde(rename = "md5-hash-from-url")]
    Md5Hash{url: String},
    #[serde(rename = "sha1-hash-from-url")]
    Sha1Hash{url: String}
}

impl Project {
    pub fn download_all_versions(&self, base_directory: impl AsRef<Path>) -> anyhow::Result<()> {
        log::info!("Downloading versions for project {}", self.name);
        let normalised_name = self.name.to_lowercase().replace(' ', "-");
        let project_directory = base_directory.as_ref().join(&normalised_name);
        std::fs::create_dir_all(&project_directory)?;
        for version in &self.versions {
            if !version.acquisition.is_available() {
                log::info!("Version {} is not available, skipping", version.format_version());
                continue;
            }
            log::info!("Downloading version {}", version.format_version());
            let version_directory = project_directory.join(version.format_version());
            if version_directory.exists() {
                log::info!("Version {} already exists, skipping", version.format_version());
                continue;
            }
            std::fs::create_dir_all(&version_directory)?;
            match version.acquisition.acquire_source_code(&version_directory) {
                Ok(_) => {
                }
                Err(e) => {
                    log::error!("Failed to download version {}: {}", version.format_version(), e);
                    std::fs::remove_dir_all(&version_directory)?;
                    return Err(e);
                }
            }
        }
        Ok(())
    }
}

impl DownloadableVersion {
    pub fn format_version(&self) -> String {
        match (self.version.patch, self.version.modifiers.as_ref()) {
            (Some(patch), Some(modifiers)) => {
                format!("{}.{}.{}.{}", self.version.major, self.version.minor, patch, modifiers)
            },
            (Some(patch), None) => {
                format!("{}.{}.{}", self.version.major, self.version.minor, patch)
            },
            (None, Some(modifiers)) => {
                format!("{}.{}.{}", self.version.major, self.version.minor, modifiers)
            },
            (None, None) => {
                format!("{}.{}", self.version.major, self.version.minor)
            }
        }
    }
}

static mut REPOSITORY_CACHE: OnceLock<HashMap<String, PathBuf>> = OnceLock::new();


impl AcquisitionMethod {
    pub fn acquire_source_code(&self, to: impl AsRef<Path>) -> anyhow::Result<()> {
        match self {
            AcquisitionMethod::GitHubTag { clone_url, tag } => {
                self.acquire_github_tag(clone_url, tag, to)
            }
            AcquisitionMethod::JarArchiveLink { url, verification } => {
                self.acquire_zip_archive_link(url, verification, to)
            }
            AcquisitionMethod::TagGzArchiveLink { url, verification } => {
                self.acquire_tar_archive_link(url, verification, to)
            }
            AcquisitionMethod::NotAvailable{} => {
                Err(anyhow::anyhow!("This version is not available"))
            }
        }
    }

    pub fn is_available(&self) -> bool {
        !matches!(self, AcquisitionMethod::NotAvailable{})
    }

    fn acquire_github_tag(&self,
                          clone_url: &str,
                          tag: &str,
                          to: impl AsRef<Path>) -> anyhow::Result<()> {
        // log::info!("Cloning repository {} to {}", clone_url, to.as_ref().display());
        // let repo = git2::Repository::clone(clone_url, to.as_ref())?;

        // Safe as long the program is single-threaded.
        let cache = unsafe {
            let _ = REPOSITORY_CACHE.get_or_init(|| HashMap::new());
            let cache = REPOSITORY_CACHE.get_mut().unwrap();
            cache
        };
        let repo_path = match cache.entry(clone_url.to_string()) {
            Entry::Occupied(e) => e.get().clone(),
            Entry::Vacant(e) => {
                let path = PathBuf::from("./github-cache");
                if !path.exists() {
                    std::fs::create_dir_all(&path)?;
                }
                let path = path.join(clone_url.rsplit_once('/').unwrap().1.trim_end_matches(".git"));
                log::info!("Cloning repository {} to {}", clone_url, path.as_path().display());
                let _ = git2::Repository::clone(clone_url, path.clone())?;
                e.insert(path).clone()
            }
        };
        let repo = git2::Repository::open(repo_path.clone())?;

        // Based on https://stackoverflow.com/a/67240436/5153960
        log::info!("Checking out tag {}", tag);

        let (object, reference) = repo.revparse_ext(tag)?;
        repo.checkout_tree(&object, None)?;
        match reference {
            // gref is an actual reference like branches or tags
            Some(gref) => repo.set_head(gref.name().expect("Oops")),
            // this is a commit, not a reference
            None => repo.set_head_detached(object.id()),
        }?;

        log::info!("Copying checked-out version to {}...", to.as_ref().display());
        Self::copy_tree(repo_path, to.as_ref())?;

        Ok(())
    }

    fn copy_tree(source: impl AsRef<Path>, destination: impl AsRef<Path>) -> anyhow::Result<()> {
        fs::create_dir_all(destination.as_ref())?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name();
            if path.is_dir() {
                Self::copy_tree(path, destination.as_ref().join(file_name))?;
            } else {
                fs::copy(path, destination.as_ref().join(file_name))?;
            }
        }
        Ok(())
    }

    fn acquire_zip_archive_link(&self,
                                url: &str,
                                verification: &[ArchiveVerificationMethod],
                                to: impl AsRef<Path>) -> anyhow::Result<()> {
        log::info!("Downloading archive from {}", url);
        let archive = self.download_archive(url)?;
        for method in verification {
            method.verify_with_error(&archive)?;
        }
        log::info!("Unpacking archive to {:?}", to.as_ref());
        let file = std::fs::File::open(&archive)?;
        let reader = std::io::BufReader::new(file);
        let mut archive = zip::ZipArchive::new(reader)?;
        archive.extract(to.as_ref())?;
        Ok(())
    }

    fn acquire_tar_archive_link(&self,
                                url: &str,
                                verification: &[ArchiveVerificationMethod],
                                to: impl AsRef<Path>) -> anyhow::Result<()> {
        log::info!("Downloading archive from {}", url);
        let archive_path = self.download_archive(url)?;
        for method in verification {
            method.verify_with_error(&archive_path)?;
        }
        log::info!("Unpacking archive to {:?}", to.as_ref());
        //let file = std::fs::File::open(&archive_path)?;
        //let reader = std::io::BufReader::new(file);
        //let decompress = flate2::read::MultiGzDecoder::new(reader);
        //let decompress = bgzip::BGZFReader::new(reader)?;
        //let mut archive = tar::Archive::new(decompress);
        //archive.unpack(to.as_ref())?;
        let in_path = archive_path.as_str();
        let out_dir = to.as_ref().as_os_str().to_str().expect("Failed");
        let _ = std::process::Command::new("tar")
            .args(["-xzf", in_path, "-C", out_dir])
            .output()?;
        Ok(())
    }

    fn download_archive(&self, url: &str) -> anyhow::Result<String> {
        let filename = url.rsplit_once('/')
            .ok_or_else(|| anyhow::anyhow!("Could not extract filename from URL: {}", url))?.1;
        // let response = reqwest::blocking::get(url)?;
        let response = reqwest::blocking::ClientBuilder::new()
            .timeout(Some(Duration::from_secs(2 * 60)))
            .build()?
            .get(url)
            .send()?;
        let data = response.bytes()?;
        let mut file = std::fs::File::create(filename)?;
        std::io::copy(&mut data.as_ref(), &mut file)?;
        Ok(filename.to_string())
    }
}


impl ArchiveVerificationMethod {
    pub fn verify_with_error(&self, file_location: impl AsRef<Path>) -> anyhow::Result<()> {
        if self.verify(file_location)? {
            Ok(())
        } else {
            match self {
                ArchiveVerificationMethod::Md5Hash { .. } => {
                    Err(anyhow::anyhow!("MD5 Hash does not match expectation"))
                }
                ArchiveVerificationMethod::Sha1Hash { .. } => {
                    Err(anyhow::anyhow!("Sha1 Hash does not match expectation"))
                }
            }
        }
    }

    pub fn verify(&self, file_location: impl AsRef<Path>) -> anyhow::Result<bool> {
        let expected = self.get_expected_hash()?;
        let actual = self.get_actual_hash(file_location)?;
        log::debug!("Expected hash: {}", expected);
        log::debug!("Actual hash: {}", actual);
        Ok(expected == actual)
    }

    fn get_expected_hash(&self) -> anyhow::Result<String> {
        let url = match self {
            ArchiveVerificationMethod::Md5Hash { url} => url,
            ArchiveVerificationMethod::Sha1Hash { url } => url
        };
        let response = reqwest::blocking::get(url)?;
        let mut expected_hash = response.text()?.to_lowercase().trim().to_string();
        if expected_hash.contains('=') {
            expected_hash = expected_hash.rsplit_once('=').unwrap().1.trim().to_string();
        }
        if expected_hash.contains('/') {
            expected_hash = expected_hash.split_once('/').unwrap().0.trim().to_string();
        }
        Ok(expected_hash)
    }

    fn get_actual_hash(&self, location: impl AsRef<Path>) -> anyhow::Result<String> {
        match self {
            ArchiveVerificationMethod::Md5Hash { .. } => {
                self.get_md5_hash(location)
            },
            ArchiveVerificationMethod::Sha1Hash { .. } => {
                self.get_sha1_hash(location)
            }
        }
    }

    fn get_md5_hash(&self, location: impl AsRef<Path>) -> anyhow::Result<String> {
        let mut hasher = md5::Context::new();
        let file = std::fs::File::open(location)?;
        let mut reader = std::io::BufReader::with_capacity(1024, &file);
        loop {
            let chunk = reader.fill_buf()?;
            if chunk.is_empty() {
                break;
            }
            hasher.consume(chunk);
            let len = chunk.len();
            reader.consume(len);
        }
        Ok(format!("{:x}", hasher.compute()))
    }

    fn get_sha1_hash(&self, location: impl AsRef<Path>) -> anyhow::Result<String> {
        let mut hasher = sha1_smol::Sha1::new();
        let file = std::fs::File::open(location)?;
        let mut reader = std::io::BufReader::with_capacity(1024, &file);
        loop {
            let chunk = reader.fill_buf()?;
            if chunk.is_empty() {
                break;
            }
            hasher.update(chunk);
            let len = chunk.len();
            reader.consume(len);
        }
        Ok(hasher.hexdigest())
    }
}
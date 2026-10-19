use anyhow::{bail, Context};
use clap::Args;
use std::path::{Path, PathBuf};
use tracing::info;

use super::{InputProvider, ProviderKind};
use crate::{client::InputGenClient, common::fixtures_from_path, processor::ProcessingTracker};

/// Name of the fixture archive published with every `tests-zkevm@<tag>` release.
const FIXTURES_ARCHIVE: &str = "fixtures_zkevm.tar.gz";

#[derive(Debug, Clone, Args)]
pub struct EestProvider {
    /// tests-zkevm release tag to download fixtures from (e.g. "v21.0.1").
    #[arg(short, long, conflicts_with = "eest_fixtures_path")]
    tag: Option<String>,

    /// Local directory of EEST fixture JSON files (unpacked archive). If not
    /// provided, --tag is required.
    #[arg(short = 'p', long, conflicts_with = "tag")]
    eest_fixtures_path: Option<PathBuf>,

    /// Directory where downloaded fixture archives are cached.
    #[arg(long, default_value_os_t = default_cache_dir())]
    cache_dir: PathBuf,

    /// Include only fixtures whose name contains one of the provided strings.
    #[arg(short, long)]
    include: Option<Vec<String>>,

    /// Exclude fixtures whose name contains one of the provided strings.
    #[arg(short, long)]
    exclude: Option<Vec<String>>,
}

fn default_cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
        .unwrap_or_else(std::env::temp_dir)
        .join("zisk-eth-client")
        .join("eest")
}

#[async_trait::async_trait]
impl InputProvider for EestProvider {
    fn kind(&self) -> ProviderKind {
        ProviderKind::Eest
    }

    async fn generate_inputs(
        &self,
        client: &dyn InputGenClient,
        output: &Path,
    ) -> anyhow::Result<()> {
        if !client.supports_provider(self.kind()) {
            bail!("{} doesn't support EEST provider", client.display_name());
        }

        let fixtures_dir = match (&self.tag, &self.eest_fixtures_path) {
            (Some(tag), None) => {
                info!("Using tests-zkevm release tag: {}", tag);
                self.ensure_fixtures(tag).await?
            }
            (None, Some(dir)) => {
                info!("Using local EEST fixtures from: {}", dir.display());
                dir.clone()
            }
            _ => bail!("either --tag or --eest-fixtures-path is required"),
        };

        let fixtures = fixtures_from_path(&fixtures_dir)?;
        let total = fixtures.len();
        let fixtures: Vec<_> = fixtures
            .into_iter()
            .filter(|f| {
                self.include
                    .as_ref()
                    .is_none_or(|inc| inc.iter().any(|s| f.name.contains(s)))
            })
            .filter(|f| {
                self.exclude
                    .as_ref()
                    .is_none_or(|exc| !exc.iter().any(|s| f.name.contains(s)))
            })
            .collect();
        info!(
            "Loaded {} stateless fixtures ({} selected), converting to ZisK inputs...",
            total,
            fixtures.len()
        );

        let mut tracker = ProcessingTracker::new(client.display_name());
        for fixture in &fixtures {
            let name = format!("EEST \"{}\"", fixture.name);
            match client.process_fixture(fixture, output) {
                Ok(_) => tracker.record_success(&name),
                Err(e) => tracker.record_error(&name, &e),
            }
        }
        tracker.log_summary();

        if tracker.error_count() > 0 {
            bail!("{} fixture(s) failed", tracker.error_count());
        }
        Ok(())
    }
}

impl EestProvider {
    /// Download and unpack the release archive once; return the fixtures dir.
    async fn ensure_fixtures(&self, tag: &str) -> anyhow::Result<PathBuf> {
        let tag = tag.strip_prefix("tests-zkevm@").unwrap_or(tag);
        let dir = self.cache_dir.join(tag);
        if dir.is_dir() {
            info!("Using cached fixtures at {}", dir.display());
            return Ok(dir);
        }

        let url = format!(
            "https://github.com/ethereum/execution-specs/releases/download/tests-zkevm@{tag}/{FIXTURES_ARCHIVE}"
        );
        info!("Downloading {url}");
        let bytes = reqwest::get(&url)
            .await
            .and_then(|r| r.error_for_status())
            .with_context(|| format!("Failed to download {url}"))?
            .bytes()
            .await
            .context("Failed to read the fixture archive")?;

        std::fs::create_dir_all(&self.cache_dir)?;
        let staging = tempfile::tempdir_in(&self.cache_dir)?;
        tar::Archive::new(flate2::read::GzDecoder::new(bytes.as_ref()))
            .unpack(staging.path())
            .context("Failed to unpack the fixture archive")?;
        // The archive has a single `fixtures/` root.
        let root = staging.path().join("fixtures");
        let root = if root.is_dir() {
            root
        } else {
            staging.path().to_path_buf()
        };
        std::fs::rename(&root, &dir)
            .with_context(|| format!("Failed to move fixtures to {}", dir.display()))?;
        info!("Unpacked fixtures to {}", dir.display());
        Ok(dir)
    }
}

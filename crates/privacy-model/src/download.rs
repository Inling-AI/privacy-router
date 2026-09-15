use crate::{Artifact, ModelFile};
use futures_util::future::join_all;
use reqwest::{Client, Url, header};
use sha2::{Digest, Sha256};
use std::{
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("model file I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("model download: {0}")]
    Http(#[from] reqwest::Error),
    #[error("invalid provider URL: {0}")]
    Url(String),
    #[error("{file} checksum/size mismatch; existing files are never overwritten")]
    Integrity { file: String },
    #[error("no usable download provider: {0}")]
    Unavailable(String),
    #[error("{0}")]
    Transfer(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// A provider only resolves artifacts. Measurement, validation and installation are shared.
pub trait DownloadBackend: Send + Sync {
    fn name(&self) -> &str;
    fn url(&self, file: ModelFile) -> Result<Url>;
}

#[derive(Debug, Clone, Copy, Display, EnumString, EnumIter)]
pub enum Hub {
    #[strum(serialize = "huggingface")]
    HuggingFace,
    #[strum(serialize = "modelscope")]
    ModelScope,
}

impl Hub {
    pub fn all() -> impl Iterator<Item = Self> {
        Self::iter()
    }
}

/// Pinned repositories on each hub contain the exact same artifact bytes.
pub struct Repository {
    hub: Hub,
    endpoint: Url,
}

impl Repository {
    pub fn new(hub: Hub) -> Self {
        let endpoint = match hub {
            Hub::HuggingFace => "https://huggingface.co/",
            Hub::ModelScope => "https://modelscope.cn/",
        };
        Self {
            hub,
            endpoint: Url::parse(endpoint).expect("built-in endpoint"),
        }
    }

    /// Supports a hub-compatible mirror or a local HTTP server.
    pub fn with_endpoint(mut self, endpoint: Url) -> Self {
        self.endpoint = endpoint;
        self
    }
}

impl DownloadBackend for Repository {
    fn name(&self) -> &str {
        match self.hub {
            Hub::HuggingFace => "Hugging Face",
            Hub::ModelScope => "ModelScope",
        }
    }

    fn url(&self, file: ModelFile) -> Result<Url> {
        let mut url = self.endpoint.clone();
        url.set_query(None);
        url.set_fragment(None);
        match self.hub {
            Hub::HuggingFace => {
                url.set_path(&format!(
                    "/openai/privacy-filter/resolve/7ffa9a043d54d1be65afb281eddf0ffbe629385b/{}",
                    file.as_ref()
                ));
                url.query_pairs_mut().append_pair("download", "true");
            }
            Hub::ModelScope => {
                url.set_path("/api/v1/models/openai-mirror/privacy-filter/repo");
                url.query_pairs_mut()
                    .append_pair("Revision", "5920bcda7113d68477525fcded470c043ced192d")
                    .append_pair("FilePath", file.as_ref());
            }
        }
        Ok(url)
    }
}

/// A bounded benchmark on the actual weights delivery path, including redirects.
#[derive(Debug, Clone, Copy)]
pub struct ProbeOptions {
    pub bytes: usize,
    pub timeout: Duration,
}

impl Default for ProbeOptions {
    fn default() -> Self {
        Self {
            bytes: 512 * 1024,
            timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Measurement {
    pub provider: String,
    pub first_byte: Duration,
    pub elapsed: Duration,
    pub bytes: usize,
}

impl Measurement {
    pub fn bytes_per_second(&self) -> f64 {
        self.bytes as f64 / self.elapsed.as_secs_f64().max(f64::EPSILON)
    }
}

#[derive(Debug)]
pub enum Event<'a> {
    Checking(ModelFile),
    Probing,
    Measured(&'a Measurement),
    Rejected {
        provider: &'a str,
        reason: &'a str,
    },
    Selected(&'a str),
    Downloading {
        file: ModelFile,
        provider: &'a str,
        bytes: u64,
        total: u64,
    },
    Ready(ModelFile),
}

struct Candidate {
    backend: Arc<dyn DownloadBackend>,
    measurement: Measurement,
}

/// Constructed only after every provider's pre-download benchmark has finished.
pub struct RankedBackends {
    candidates: Vec<Candidate>,
}

impl RankedBackends {
    pub fn measurements(&self) -> impl Iterator<Item = &Measurement> {
        self.candidates
            .iter()
            .map(|candidate| &candidate.measurement)
    }
}

pub struct Downloader {
    client: Client,
    backends: Vec<Arc<dyn DownloadBackend>>,
    probe: ProbeOptions,
}

impl Downloader {
    pub fn new(backends: Vec<Arc<dyn DownloadBackend>>) -> Result<Self> {
        let client = Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .read_timeout(Duration::from_secs(30))
            .user_agent(concat!("privacy-router/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            client,
            backends,
            probe: ProbeOptions::default(),
        })
    }

    pub fn with_probe_options(mut self, options: ProbeOptions) -> Self {
        self.probe = options;
        self
    }

    pub async fn benchmark(&self, mut observe: impl FnMut(Event<'_>)) -> Result<RankedBackends> {
        if self.probe.bytes == 0 || self.probe.timeout.is_zero() {
            return Err(Error::Transfer(
                "probe size and timeout must be positive".into(),
            ));
        }
        observe(Event::Probing);
        // Complete the entire preflight phase before exposing any download candidate.
        let results = join_all(self.backends.iter().map(|backend| async {
            let result =
                tokio::time::timeout(self.probe.timeout, self.probe(backend.as_ref())).await;
            let result =
                result.unwrap_or_else(|_| Err(Error::Transfer("speed test timed out".into())));
            (backend.clone(), result)
        }))
        .await;
        let mut candidates = Vec::new();
        let mut errors = Vec::new();
        for (backend, result) in results {
            match result {
                Ok(measurement) => {
                    observe(Event::Measured(&measurement));
                    candidates.push(Candidate {
                        backend,
                        measurement,
                    });
                }
                Err(error) => {
                    let reason = error.to_string();
                    observe(Event::Rejected {
                        provider: backend.name(),
                        reason: &reason,
                    });
                    errors.push(format!("{}: {reason}", backend.name()));
                }
            }
        }
        candidates.sort_by_key(|candidate| candidate.measurement.elapsed);
        if candidates.is_empty() {
            return Err(Error::Unavailable(errors.join("; ")));
        }
        observe(Event::Selected(candidates[0].backend.name()));
        Ok(RankedBackends { candidates })
    }

    async fn probe(&self, backend: &dyn DownloadBackend) -> Result<Measurement> {
        let started = Instant::now();
        let mut response = self
            .client
            .get(backend.url(ModelFile::Weights)?)
            .header(header::RANGE, format!("bytes=0-{}", self.probe.bytes - 1))
            .header(header::ACCEPT_ENCODING, "identity")
            .send()
            .await?
            .error_for_status()?;
        let mut bytes = 0;
        let mut first_byte = None;
        while bytes < self.probe.bytes {
            let chunk = response
                .chunk()
                .await?
                .ok_or_else(|| Error::Transfer("speed-test sample was truncated".into()))?;
            if chunk.is_empty() {
                continue;
            }
            first_byte.get_or_insert_with(|| started.elapsed());
            bytes += chunk.len().min(self.probe.bytes - bytes);
        }
        // Drop the response at the sample boundary even if a provider ignores Range.
        Ok(Measurement {
            provider: backend.name().into(),
            first_byte: first_byte.unwrap(),
            elapsed: started.elapsed(),
            bytes,
        })
    }

    /// Verify local files first, then benchmark all providers, then download missing artifacts.
    pub async fn ensure(
        &self,
        directory: &Path,
        artifacts: &[Artifact],
        mut observe: impl FnMut(Event<'_>),
    ) -> Result<()> {
        let mut missing = Vec::new();
        for artifact in artifacts {
            observe(Event::Checking(artifact.file));
            match Self::verify(directory, artifact).await {
                Ok(()) => observe(Event::Ready(artifact.file)),
                Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                    missing.push(artifact)
                }
                Err(error) => return Err(error),
            }
        }
        if missing.is_empty() {
            return Ok(());
        }
        let ranked = self.benchmark(&mut observe).await?;
        tokio::fs::create_dir_all(directory).await?;
        for artifact in missing {
            self.download(directory, artifact, &ranked, &mut observe)
                .await?;
            observe(Event::Ready(artifact.file));
        }
        Ok(())
    }

    pub async fn verify(directory: &Path, artifact: &Artifact) -> Result<()> {
        let mut file = tokio::fs::File::open(artifact.file.path(directory)).await?;
        let mut digest = Sha256::new();
        let mut buffer = vec![0; 64 * 1024];
        let mut size = 0;
        loop {
            let read = file.read(&mut buffer).await?;
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
            size += read as u64;
        }
        artifact.validate(size, digest)
    }

    async fn download(
        &self,
        directory: &Path,
        artifact: &Artifact,
        ranked: &RankedBackends,
        observe: &mut impl FnMut(Event<'_>),
    ) -> Result<()> {
        let mut failures = Vec::new();
        for (index, candidate) in ranked.candidates.iter().enumerate() {
            let backend = candidate.backend.as_ref();
            if index > 0 {
                observe(Event::Selected(backend.name()));
            }
            match self.transfer(directory, artifact, backend, observe).await {
                Ok(()) => return Ok(()),
                Err(Error::Io(error)) => return Err(Error::Io(error)),
                Err(error) => {
                    let reason = error.to_string();
                    observe(Event::Rejected {
                        provider: backend.name(),
                        reason: &reason,
                    });
                    failures.push(format!("{}: {reason}", backend.name()));
                }
            }
        }
        Err(Error::Unavailable(failures.join("; ")))
    }

    async fn transfer(
        &self,
        directory: &Path,
        artifact: &Artifact,
        backend: &dyn DownloadBackend,
        observe: &mut impl FnMut(Event<'_>),
    ) -> Result<()> {
        observe(Event::Downloading {
            file: artifact.file,
            provider: backend.name(),
            bytes: 0,
            total: artifact.size,
        });
        let mut response = self
            .client
            .get(backend.url(artifact.file)?)
            .header(header::ACCEPT_ENCODING, "identity")
            .send()
            .await?
            .error_for_status()?;
        let temporary = tempfile::NamedTempFile::new_in(directory)?;
        let mut output = tokio::fs::File::from_std(temporary.as_file().try_clone()?);
        let mut digest = Sha256::new();
        let mut size = 0;
        while let Some(chunk) = response.chunk().await? {
            size += chunk.len() as u64;
            if size > artifact.size {
                return Err(Error::Integrity {
                    file: artifact.file.as_ref().into(),
                });
            }
            output.write_all(&chunk).await?;
            digest.update(&chunk);
            observe(Event::Downloading {
                file: artifact.file,
                provider: backend.name(),
                bytes: size,
                total: artifact.size,
            });
        }
        artifact.validate(size, digest)?;
        output.flush().await?;
        output.sync_all().await?;
        drop(output);
        // A concurrent bootstrap may have installed the file; never replace its destination.
        match temporary.persist_noclobber(artifact.file.path(directory)) {
            Ok(_) => Ok(()),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
                Self::verify(directory, artifact).await
            }
            Err(error) => Err(Error::Io(error.error)),
        }
    }
}

impl Artifact {
    fn validate(&self, size: u64, digest: Sha256) -> Result<()> {
        if size != self.size
            || digest
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
                != self.sha256
        {
            return Err(Error::Integrity {
                file: self.file.as_ref().into(),
            });
        }
        Ok(())
    }
}

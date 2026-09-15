use axum::{
    Router,
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode},
};
use futures_util::StreamExt;
use privacy_model::{
    Artifact, DownloadBackend, Downloader, Error, Hub, ModelFile, ProbeOptions, Repository,
};
use rstest::{fixture, rstest};
use sha2::{Digest, Sha256};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

const CONTENT: &[u8] = b"verified model configuration";

#[fixture]
fn artifact() -> Artifact {
    Artifact {
        file: ModelFile::Config,
        sha256: Sha256::digest(CONTENT)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        size: CONTENT.len() as u64,
    }
}

#[derive(Clone, Copy)]
enum Behavior {
    Healthy,
    Corrupt,
    Broken,
    Unavailable,
    ShortProbe,
    IgnoresRange,
    Stalled,
}

#[derive(Clone)]
struct ServerState {
    behavior: Behavior,
    probe_delay: Duration,
    probes_completed: Arc<AtomicUsize>,
    required_probes: usize,
    requests: Arc<Mutex<Vec<String>>>,
}

struct TestServer {
    state: ServerState,
    endpoint: reqwest::Url,
    task: tokio::task::JoinHandle<()>,
}

impl TestServer {
    async fn start(
        behavior: Behavior,
        delay: Duration,
        probes: Arc<AtomicUsize>,
        required: usize,
    ) -> Self {
        let state = ServerState {
            behavior,
            probe_delay: delay,
            probes_completed: probes,
            required_probes: required,
            requests: Arc::default(),
        };
        let app = Router::new()
            .fallback(Self::respond)
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/", listener.local_addr().unwrap())
            .parse()
            .unwrap();
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        Self {
            state,
            endpoint,
            task,
        }
    }

    async fn respond(State(state): State<ServerState>, request: Request<Body>) -> Response<Body> {
        state
            .requests
            .lock()
            .unwrap()
            .push(request.uri().to_string());
        if request.headers().contains_key("range") {
            tokio::time::sleep(state.probe_delay).await;
            state.probes_completed.fetch_add(1, Ordering::SeqCst);
            if matches!(state.behavior, Behavior::Unavailable) {
                return Response::builder()
                    .status(StatusCode::SERVICE_UNAVAILABLE)
                    .body(Body::empty())
                    .unwrap();
            }
            let length = if matches!(state.behavior, Behavior::ShortProbe) {
                8
            } else {
                1024
            };
            let status = if matches!(state.behavior, Behavior::IgnoresRange) {
                StatusCode::OK
            } else {
                StatusCode::PARTIAL_CONTENT
            };
            return Response::builder()
                .status(status)
                .body(Body::from(vec![7_u8; length]))
                .unwrap();
        }
        // This is an external protocol requirement: no artifact GET before all speed tests finish.
        if state.probes_completed.load(Ordering::SeqCst) < state.required_probes {
            return Response::builder()
                .status(StatusCode::CONFLICT)
                .body(Body::empty())
                .unwrap();
        }
        let body = match state.behavior {
            Behavior::Corrupt => Body::from(vec![0; CONTENT.len()]),
            Behavior::Broken => Body::from_stream(futures_util::stream::iter([
                Ok(axum::body::Bytes::from_static(b"partial")),
                Err(std::io::Error::other("connection interrupted")),
            ])),
            Behavior::Stalled => Body::from_stream(
                futures_util::stream::once(async {
                    Ok::<_, std::io::Error>(axum::body::Bytes::from_static(b"partial"))
                })
                .chain(futures_util::stream::pending()),
            ),
            _ => Body::from(CONTENT),
        };
        Response::new(body)
    }

    fn backend(&self, hub: Hub) -> Arc<dyn DownloadBackend> {
        Arc::new(Repository::new(hub).with_endpoint(self.endpoint.clone()))
    }

    fn downloader(backends: Vec<Arc<dyn DownloadBackend>>) -> Downloader {
        Downloader::new(backends)
            .unwrap()
            .with_probe_options(ProbeOptions {
                bytes: 128,
                timeout: Duration::from_millis(500),
            })
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[rstest]
#[tokio::test]
async fn speed_test_finishes_for_all_providers_before_fastest_download_starts(artifact: Artifact) {
    let probes = Arc::new(AtomicUsize::new(0));
    let slow = TestServer::start(
        Behavior::Healthy,
        Duration::from_millis(100),
        probes.clone(),
        2,
    )
    .await;
    let fast = TestServer::start(Behavior::Healthy, Duration::ZERO, probes, 2).await;
    let directory = tempfile::tempdir().unwrap();
    let downloader = TestServer::downloader(vec![
        slow.backend(Hub::HuggingFace),
        fast.backend(Hub::ModelScope),
    ]);
    downloader
        .ensure(directory.path(), &[artifact], |_| {})
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(ModelFile::Config.path(directory.path())).unwrap(),
        CONTENT
    );
    assert_eq!(
        slow.state.requests.lock().unwrap().len(),
        1,
        "slow provider only receives its benchmark request"
    );
    assert_eq!(
        fast.state.requests.lock().unwrap().len(),
        2,
        "fast provider receives the artifact GET after preflight"
    );
}

#[rstest]
#[case(Behavior::Corrupt)]
#[case(Behavior::Broken)]
#[tokio::test]
async fn failed_download_falls_back_to_next_ranked_provider(
    artifact: Artifact,
    #[case] failure: Behavior,
) {
    let probes = Arc::new(AtomicUsize::new(0));
    let failed = TestServer::start(failure, Duration::ZERO, probes.clone(), 2).await;
    let healthy = TestServer::start(Behavior::Healthy, Duration::from_millis(100), probes, 2).await;
    let directory = tempfile::tempdir().unwrap();
    TestServer::downloader(vec![
        failed.backend(Hub::HuggingFace),
        healthy.backend(Hub::ModelScope),
    ])
    .ensure(directory.path(), &[artifact], |_| {})
    .await
    .unwrap();
    assert_eq!(
        std::fs::read(ModelFile::Config.path(directory.path())).unwrap(),
        CONTENT
    );
    assert_eq!(failed.state.requests.lock().unwrap().len(), 2);
    assert_eq!(healthy.state.requests.lock().unwrap().len(), 2);
    assert_eq!(
        std::fs::read_dir(directory.path()).unwrap().count(),
        1,
        "failed temporary files are removed"
    );
}

#[rstest]
#[case(Behavior::Unavailable)]
#[case(Behavior::ShortProbe)]
#[tokio::test]
async fn failed_speed_tests_are_excluded_from_downloads(
    artifact: Artifact,
    #[case] failure: Behavior,
) {
    let probes = Arc::new(AtomicUsize::new(0));
    let failed = TestServer::start(failure, Duration::ZERO, probes.clone(), 2).await;
    let healthy = TestServer::start(Behavior::Healthy, Duration::ZERO, probes, 2).await;
    let directory = tempfile::tempdir().unwrap();
    TestServer::downloader(vec![
        failed.backend(Hub::HuggingFace),
        healthy.backend(Hub::ModelScope),
    ])
    .ensure(directory.path(), &[artifact], |_| {})
    .await
    .unwrap();
    assert_eq!(
        std::fs::read(ModelFile::Config.path(directory.path())).unwrap(),
        CONTENT
    );
    assert_eq!(failed.state.requests.lock().unwrap().len(), 1);
}

#[rstest]
#[tokio::test]
async fn verified_files_work_offline_without_a_speed_test(artifact: Artifact) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(ModelFile::Config.path(directory.path()), CONTENT).unwrap();
    TestServer::downloader(vec![])
        .ensure(directory.path(), &[artifact], |_| {})
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(ModelFile::Config.path(directory.path())).unwrap(),
        CONTENT
    );
}

#[rstest]
#[tokio::test]
async fn corrupt_existing_files_are_preserved_and_rejected(artifact: Artifact) {
    let directory = tempfile::tempdir().unwrap();
    let path = ModelFile::Config.path(directory.path());
    std::fs::write(&path, b"existing user data").unwrap();
    let result = TestServer::downloader(vec![])
        .ensure(directory.path(), &[artifact], |_| {})
        .await;
    assert!(matches!(result, Err(Error::Integrity { .. })));
    assert_eq!(std::fs::read(path).unwrap(), b"existing user data");
}

#[rstest]
#[case(Behavior::Corrupt)]
#[case(Behavior::Broken)]
#[tokio::test]
async fn incomplete_or_unverified_artifacts_are_never_installed(
    artifact: Artifact,
    #[case] failure: Behavior,
) {
    let server = TestServer::start(failure, Duration::ZERO, Arc::new(AtomicUsize::new(0)), 1).await;
    let directory = tempfile::tempdir().unwrap();
    let result = TestServer::downloader(vec![server.backend(Hub::HuggingFace)])
        .ensure(directory.path(), &[artifact], |_| {})
        .await;
    assert!(matches!(result, Err(Error::Unavailable(_))));
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[rstest]
#[tokio::test]
async fn a_timed_out_provider_cannot_block_other_candidates() {
    let probes = Arc::new(AtomicUsize::new(0));
    let slow = TestServer::start(
        Behavior::Healthy,
        Duration::from_secs(60),
        probes.clone(),
        0,
    )
    .await;
    let fast = TestServer::start(Behavior::IgnoresRange, Duration::ZERO, probes, 0).await;
    let ranked = TestServer::downloader(vec![
        slow.backend(Hub::HuggingFace),
        fast.backend(Hub::ModelScope),
    ])
    .benchmark(|_| {})
    .await
    .unwrap();
    let measurements: Vec<_> = ranked.measurements().collect();
    assert_eq!(measurements.len(), 1);
    assert_eq!(measurements[0].provider, "ModelScope");
    assert_eq!(measurements[0].bytes, 128);
}

#[rstest]
#[tokio::test]
async fn no_healthy_provider_does_not_create_model_files(artifact: Artifact) {
    let server = TestServer::start(
        Behavior::Unavailable,
        Duration::ZERO,
        Arc::new(AtomicUsize::new(0)),
        1,
    )
    .await;
    let directory = tempfile::tempdir().unwrap();
    let result = TestServer::downloader(vec![server.backend(Hub::HuggingFace)])
        .ensure(directory.path(), &[artifact], |_| {})
        .await;
    assert!(matches!(result, Err(Error::Unavailable(_))));
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[rstest]
#[tokio::test]
async fn cancellation_cleans_up_an_incomplete_artifact(artifact: Artifact) {
    let server = TestServer::start(
        Behavior::Stalled,
        Duration::ZERO,
        Arc::new(AtomicUsize::new(0)),
        1,
    )
    .await;
    let directory = tempfile::tempdir().unwrap();
    let destination = directory.path().to_owned();
    let downloader = TestServer::downloader(vec![server.backend(Hub::ModelScope)]);
    let (sent, received) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
        let mut sent = Some(sent);
        downloader
            .ensure(&destination, &[artifact], |event| {
                if matches!(event, privacy_model::Event::Downloading { bytes, .. } if bytes > 0)
                    && let Some(sent) = sent.take()
                {
                    let _ = sent.send(());
                }
            })
            .await
    });
    tokio::time::timeout(Duration::from_secs(1), received)
        .await
        .unwrap()
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
}

#[rstest]
#[tokio::test]
async fn concurrent_bootstraps_publish_one_verified_file(artifact: Artifact) {
    let server = TestServer::start(
        Behavior::Healthy,
        Duration::ZERO,
        Arc::new(AtomicUsize::new(0)),
        0,
    )
    .await;
    let directory = tempfile::tempdir().unwrap();
    let downloader = TestServer::downloader(vec![server.backend(Hub::HuggingFace)]);
    let artifacts = [artifact];
    let (first, second) = tokio::join!(
        downloader.ensure(directory.path(), &artifacts, |_| {}),
        downloader.ensure(directory.path(), &artifacts, |_| {}),
    );
    first.unwrap();
    second.unwrap();
    assert_eq!(
        std::fs::read(ModelFile::Config.path(directory.path())).unwrap(),
        CONTENT
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

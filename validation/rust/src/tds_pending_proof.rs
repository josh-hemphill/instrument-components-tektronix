//! Real pending host IO behind the actual new factories; no device progress or hardware.
use crate::{generated_cases, tds_fixture};
use instrument_api::{catalog::*, *};
use instrument_runtime::*;
use instrument_scpi::*;
use instrument_testkit::*;
use std::{
    collections::BTreeSet,
    future::Future,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll, Waker},
    time::{Duration, Instant},
};

fn cases() -> std::collections::BTreeMap<&'static str, CatalogCase> {
    generated_cases::cases()
        .into_iter()
        .map(|c| (c.id, c))
        .collect()
}
fn candidate(_id: &str) -> Candidate {
    Candidate {
        endpoint: Endpoint {
            scheme: "scpi".into(),
            address: "fixture".into(),
        },
        identity: Identity::default(),
    }
}
fn selection(id: &str) -> Selection {
    Selection {
        driver_id: Some(id.split('|').next().unwrap().into()),
        ..Selection::default()
    }
}
#[derive(Clone)]
struct Host {
    script: Script,
    pending_at: usize,
    pending_message: String,
    pending_write: bool,
    response: String,
    deadline: bool,
    native_only: bool,
    calls: Arc<AtomicUsize>,
    active: Arc<AtomicUsize>,
    release: Arc<AtomicBool>,
}
impl Host {
    fn new(
        case: &CatalogCase,
        pending_at: usize,
        owned: bool,
        deadline: bool,
        native_only: bool,
        second: Option<&CatalogCase>,
    ) -> Self {
        assert_eq!(case.calls.len(), 1);
        let write = matches!(case.script[pending_at], ScriptStep::Write(_));
        let (pending_message, response) = match &case.script[pending_at] {
            ScriptStep::Write(s) => (s.clone(), String::new()),
            ScriptStep::Query(s, Ok(r)) => (s.clone(), r.clone()),
            _ => panic!("Expected successful oracle step"),
        };
        let mut prefix = case.script[..pending_at].to_vec();
        if let Some(second) = second {
            prefix.extend(
                case.script[pending_at + 1..]
                    .iter()
                    .filter(|s| !matches!(s, ScriptStep::Close(_)))
                    .cloned(),
            );
            prefix.extend(
                second
                    .script
                    .iter()
                    .filter(|s| !matches!(s, ScriptStep::Close(_)))
                    .cloned(),
            );
        }
        if owned {
            prefix.push(ScriptStep::Close(Ok(())));
        }
        Self {
            script: Script::new(prefix),
            pending_at,
            pending_message,
            pending_write: write,
            response,
            deadline,
            native_only,
            calls: Arc::new(AtomicUsize::new(0)),
            active: Arc::new(AtomicUsize::new(0)),
            release: Arc::new(AtomicBool::new(false)),
        }
    }
    fn step(&self, message: &str, write: bool) -> bool {
        let index = self.calls.fetch_add(1, Ordering::SeqCst);
        if index == self.pending_at {
            assert_eq!(message, self.pending_message);
            assert_eq!(write, self.pending_write);
            true
        } else {
            false
        }
    }
    fn sync_pending(&self) -> Result<String> {
        self.active.fetch_add(1, Ordering::SeqCst);
        let _guard = Active(self.active.clone());
        let (_sender, receiver) = std::sync::mpsc::channel::<String>();
        receiver
            .recv_timeout(Duration::from_millis(20))
            .map_err(|_| DriverError::new(ErrorKind::Timeout, "Independent pending-host deadline"))
    }
    async fn pending(&self) -> Result<String> {
        self.active.fetch_add(1, Ordering::SeqCst);
        let _guard = Active(self.active.clone());
        if self.deadline {
            tokio::time::timeout(Duration::from_millis(20), std::future::pending::<String>())
                .await
                .map_err(|_| {
                    DriverError::new(ErrorKind::Timeout, "Independent pending-host deadline")
                })
        } else {
            std::future::poll_fn(|_| {
                if self.release.load(Ordering::SeqCst) {
                    Poll::Ready(Ok(self.response.clone()))
                } else {
                    Poll::Pending
                }
            })
            .await
        }
    }
}
struct Active(Arc<AtomicUsize>);
impl Drop for Active {
    fn drop(&mut self) {
        assert_eq!(self.0.fetch_sub(1, Ordering::SeqCst), 1);
    }
}
struct SyncIo {
    host: Host,
    inner: Box<dyn MessageIo>,
}
struct NativeIo {
    host: Host,
    inner: Box<dyn AsyncMessageIo>,
}
impl Connector for Host {
    fn open(&self, context: &OpenContext<'_>) -> Result<Box<dyn MessageIo>> {
        assert!(!self.native_only, "Native path used sync acquisition");
        Ok(Box::new(SyncIo {
            host: self.clone(),
            inner: Connector::open(&self.script, context)?,
        }))
    }
}
impl AsyncConnector for Host {
    fn open<'a>(
        &'a self,
        context: &'a OpenContext<'_>,
    ) -> DriverFuture<'a, Box<dyn AsyncMessageIo>> {
        Box::pin(async move {
            Ok(Box::new(NativeIo {
                host: self.clone(),
                inner: AsyncConnector::open(&self.script, context).await?,
            }) as Box<dyn AsyncMessageIo>)
        })
    }
}
impl MessageIo for SyncIo {
    fn write(&mut self, message: &str) -> Result<()> {
        assert!(!self.host.native_only);
        if self.host.step(message, true) {
            self.host.sync_pending().map(|_| ())
        } else {
            self.inner.write(message)
        }
    }
    fn query(&mut self, message: &str) -> Result<String> {
        assert!(!self.host.native_only);
        if self.host.step(message, false) {
            self.host.sync_pending()
        } else {
            self.inner.query(message)
        }
    }
    fn close(&mut self) -> Result<()> {
        self.inner.close()
    }
}
impl AsyncMessageIo for NativeIo {
    fn write<'a>(&'a mut self, message: &'a str) -> DriverFuture<'a, ()> {
        Box::pin(async move {
            if self.host.step(message, true) {
                self.host.pending().await.map(|_| ())
            } else {
                self.inner.write(message).await
            }
        })
    }
    fn query<'a>(&'a mut self, message: &'a str) -> DriverFuture<'a, String> {
        Box::pin(async move {
            if self.host.step(message, false) {
                self.host.pending().await
            } else {
                self.inner.query(message).await
            }
        })
    }
    fn close(&mut self) -> Result<()> {
        self.inner.close()
    }
}
fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}
fn poll_to_host<F: Future>(mut future: Pin<&mut F>, host: &Host) {
    // TestKit intentionally yields at every prefix step. Reach the genuinely
    // pending target, rather than mistaking an ordinary scheduling yield for IO.
    for _ in 0..host.pending_at + 4 {
        assert!(poll_once(future.as_mut()).is_pending());
        if host.active.load(Ordering::SeqCst) == 1 {
            return;
        }
    }
    panic!("Did not reach the pending host target");
}
pub async fn run() -> Result<()> {
    let cases = cases();
    let id = "tektronix.tds1002|E|channel-1-fresh-Vpp";
    let case = &cases[id];
    let request = case.calls[0].clone();
    for target in [":HEAD OFF", ":ACQ:STATE RUN", "*OPC?", ":MEASU:IMM:VAL?"] {
        let position = case
            .script
            .iter()
            .position(|s| match s {
                ScriptStep::Write(m) | ScriptStep::Query(m, _) => m == target,
                _ => false,
            })
            .unwrap();
        for ownership in [ConnectionOwnership::Owned, ConnectionOwnership::Borrowed] {
            let owned = ownership == ConnectionOwnership::Owned;
            let host = Host::new(case, position, owned, false, true, None);
            let registry = DriverRegistry::new(vec![tds_fixture::factory(&host, true)?])?;
            let handle = registry
                .open_native(&candidate(id), &selection(id), &BTreeSet::new(), ownership)
                .await?;
            drop(handle.invoke_async(request.clone()));
            assert_eq!(host.calls.load(Ordering::SeqCst), 0);
            let mut future = Box::pin(handle.invoke_async(request.clone()));
            poll_to_host(future.as_mut(), &host);
            assert_eq!(host.active.load(Ordering::SeqCst), 1);
            drop(future);
            assert_eq!(host.active.load(Ordering::SeqCst), 0);
            let calls = host.calls.load(Ordering::SeqCst);
            assert_eq!(
                handle.invoke_async(request.clone()).await.unwrap_err().kind,
                ErrorKind::SessionNeedsRecovery
            );
            handle.close(false).await?;
            tokio::task::yield_now().await;
            assert_eq!(host.calls.load(Ordering::SeqCst), calls);
            assert_eq!(host.script.closes(), usize::from(owned));
            host.script.assert_complete()?;
        }
        for native in [false, true] {
            let host = Host::new(case, position, true, true, native, None);
            let registry = DriverRegistry::new(vec![tds_fixture::factory(&host, true)?])?;
            let started = Instant::now();
            if native {
                let handle = registry
                    .open_native(
                        &candidate(id),
                        &selection(id),
                        &BTreeSet::new(),
                        ConnectionOwnership::Owned,
                    )
                    .await?;
                assert_eq!(
                    handle.invoke_async(request.clone()).await.unwrap_err().kind,
                    ErrorKind::Timeout
                );
                assert_eq!(
                    handle.invoke_async(request.clone()).await.unwrap_err().kind,
                    ErrorKind::SessionNeedsRecovery
                );
                handle.close(false).await?;
            } else {
                let handle = registry.open(
                    &candidate(id),
                    &selection(id),
                    &BTreeSet::new(),
                    ExecutionMode::Sync,
                    ConnectionOwnership::Owned,
                )?;
                assert_eq!(
                    handle.invoke(request.clone()).unwrap_err().kind,
                    ErrorKind::Timeout
                );
                assert_eq!(
                    handle.invoke(request.clone()).unwrap_err().kind,
                    ErrorKind::SessionNeedsRecovery
                );
                handle.close(false)?;
            }
            assert!(
                started.elapsed() >= Duration::from_millis(15)
                    && started.elapsed() < Duration::from_secs(5)
            );
            assert_eq!(host.calls.load(Ordering::SeqCst), position + 1);
            assert_eq!(host.active.load(Ordering::SeqCst), 0);
            host.script.assert_complete()?;
            assert_eq!(host.script.closes(), 1);
        }
    }
    // Pause OPC with VALUE/post-ESR still ahead: no second class-view IO may
    // interleave anywhere in the first whole measurement transaction.
    let position = case
        .script
        .iter()
        .position(|s| matches!(s,ScriptStep::Query(m,_)if m=="*OPC?"))
        .unwrap();
    let second = &cases["tektronix.tds1002|E|acquisition-Stop"];
    let host = Host::new(case, position, true, false, true, Some(second));
    let registry = DriverRegistry::new(vec![tds_fixture::factory(&host, true)?])?;
    let handle = registry
        .open_native(
            &candidate(id),
            &selection(id),
            &BTreeSet::new(),
            ConnectionOwnership::Owned,
        )
        .await?;
    let mut first = Box::pin(handle.invoke_async(request));
    poll_to_host(first.as_mut(), &host);
    let calls = host.calls.load(Ordering::SeqCst);
    let mut other = Box::pin(handle.invoke_async(second.calls[0].clone()));
    assert!(poll_once(other.as_mut()).is_pending());
    assert_eq!(host.calls.load(Ordering::SeqCst), calls);
    host.release.store(true, Ordering::SeqCst);
    assert_eq!(first.await?, case.expected.clone().unwrap());
    assert_eq!(other.await?, second.expected.clone().unwrap());
    handle.close(false).await?;
    host.script.assert_complete()?;
    assert_eq!(host.script.closes(), 1);
    assert_eq!(host.active.load(Ordering::SeqCst), 0);
    println!("PASS TDS1002 pending HEAD/RUN/OPC/VALUE: native abandonment, sync/native independent deadlines, ownership, whole-capture serialization and no automatic STOP");
    Ok(())
}

//! The server's stop (docs/face.md, 「The server」): asked by SIGTERM or an interrupt, it takes no new
//! connection, ends every open stream, and lets the calls in hand finish within a deadline.

use std::future::IntoFuture;
use std::io::Write;
use std::net::SocketAddr;
use std::time::Duration;

use axum::Router;
use axum::extract::connect_info::IntoMakeServiceWithConnectInfo;
use futures_util::{Stream, StreamExt};
use tokio::net::TcpListener;
use tokio::signal::unix::{Signal, SignalKind, signal};
use tokio::sync::watch;

/// How long the calls in hand are let finish once the server is asked to stop, unless its environment
/// names a shorter one.
pub const DRAIN: Duration = Duration::from_secs(10);

/// Whether the server has been asked to stop, which every open stream and the serve itself wait on.
#[derive(Clone, Debug)]
pub struct Stop(watch::Sender<bool>);

impl Default for Stop {
  fn default() -> Self {
    Self(watch::Sender::new(false))
  }
}

impl Stop {
  /// The server asked to stop; asked again, nothing more happens.
  pub fn stop(&self) {
    self.0.send_replace(true);
  }

  /// Ends once the server has been asked to stop, at once where it already has.
  pub async fn stopped(&self) {
    let mut asked = self.0.subscribe();
    // The sender is `self`'s own, so the wait ends only with the stop.
    let _ = asked.wait_for(|stopping| *stopping).await;
  }

  /// `stream` as it comes, ended once the server is asked to stop.
  pub fn ends<S: Stream>(&self, stream: S) -> impl Stream<Item = S::Item> + use<S> {
    let stop = self.clone();
    stream.take_until(async move { stop.stopped().await })
  }

  /// The server asked to stop when SIGTERM or an interrupt comes. Both are heard from this call on, so
  /// a signal sent once the server names its port drains it rather than ending it outright; the
  /// returned wait asks the stop.
  pub fn on_signal(self) -> impl Future<Output = ()> + Send + 'static {
    let heard = [SignalKind::terminate(), SignalKind::interrupt()].map(|kind| {
      signal(kind)
        .inspect_err(|why| eprintln!("bmo-face: a stopping signal cannot be heard: {why}"))
        .ok()
    });
    async move {
      let [terminate, interrupt] = heard;
      tokio::select! {
        () = arrival(terminate) => {}
        () = arrival(interrupt) => {}
      }
      self.stop();
      // Said once asked, so an output nobody reads any more costs the line and never the stop.
      let _ = writeln!(std::io::stdout(), "bmo-face: stopping; the calls in hand are let finish");
    }
  }
}

// Ends when the signal comes, and never for one that cannot be heard.
async fn arrival(heard: Option<Signal>) {
  match heard {
    Some(mut signal) => {
      signal.recv().await;
    }
    None => std::future::pending().await,
  }
}

/// `app` answering at `listener` until `stop` is asked, then drained: no new connection taken, each idle
/// one closed, and the calls in hand let finish within `deadline`, past which those still open end with
/// the serve.
pub async fn drain(
  listener: TcpListener,
  app: IntoMakeServiceWithConnectInfo<Router, SocketAddr>,
  stop: Stop,
  deadline: Duration,
) -> std::io::Result<()> {
  let asked = stop.clone();
  let serving = axum::serve(listener, app)
    .with_graceful_shutdown(async move { asked.stopped().await })
    .into_future();
  let cut = async {
    stop.stopped().await;
    tokio::time::sleep(deadline).await;
  };

  tokio::select! {
    served = serving => served,
    () = cut => {
      // An output nobody reads any more costs the line and never the drained server's success.
      let _ = writeln!(
        std::io::stderr(),
        "bmo-face: the calls still in hand after {}ms end with the server",
        deadline.as_millis()
      );
      Ok(())
    }
  }
}

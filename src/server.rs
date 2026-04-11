use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use http_body_util::BodyExt;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use crate::middleware::MiddlewareStack;
use crate::request;
use crate::response;
use crate::template_cache::{ReloadPolicy, TemplateCache};

pub struct Server<D> {
    middleware_stack: MiddlewareStack<D>,
    templates: TemplateCache,
    shared_data: D,
}

impl<D: Sync + Send + 'static> Server<D> {
    pub fn new(middleware_stack: MiddlewareStack<D>, reload_policy: ReloadPolicy, data: D) -> Server<D> {
        Server {
            middleware_stack,
            templates: TemplateCache::with_policy(reload_policy),
            shared_data: data,
        }
    }

    pub fn serve(self,
                 addr: &str,
                 _keep_alive_timeout: Option<Duration>,
                 _thread_count: Option<usize>)
                 -> Result<ListeningServer, Box<dyn std::error::Error + Send + Sync>> {
        let arc = Arc::new(self);
        let (addr_tx, addr_rx) = oneshot::channel();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

        let addr_owned = addr.to_string();
        let server_arc = arc.clone();
        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            rt.block_on(async move {
                let listener = TcpListener::bind(&addr_owned).await
                    .expect("Failed to bind address");
                let local_addr = listener.local_addr().expect("Failed to get local address");
                let _ = addr_tx.send(local_addr);

                tokio::select! {
                    _ = async {
                        loop {
                            let (stream, _) = match listener.accept().await {
                                Ok(conn) => conn,
                                Err(e) => {
                                    eprintln!("Accept error: {}", e);
                                    continue;
                                }
                            };
                            let io = TokioIo::new(stream);
                            let server = server_arc.clone();

                            tokio::task::spawn(async move {
                                let service = service_fn(move |req: hyper::Request<Incoming>| {
                                    let server = server.clone();
                                    async move {
                                        handle_request(server, req).await
                                    }
                                });

                                if let Err(e) = http1::Builder::new()
                                    .serve_connection(io, service)
                                    .await
                                {
                                    eprintln!("Connection error: {}", e);
                                }
                            });
                        }
                    } => {},
                    _ = shutdown_rx => {},
                };
            });
        });

        let socket = addr_rx.blocking_recv()
            .map_err(|_| "Failed to receive bound address")?;

        Ok(ListeningServer {
            socket,
            shutdown: Some(shutdown_tx),
            handle: Some(handle),
        })
    }
}

async fn handle_request<D: Sync + Send + 'static>(
    server: Arc<Server<D>>,
    req: hyper::Request<Incoming>,
) -> Result<hyper::Response<http_body_util::Full<bytes::Bytes>>, std::convert::Infallible> {
    let (parts, body) = req.into_parts();
    let method = parts.method;
    let uri = parts.uri.to_string();
    let headers = parts.headers;

    // Collect the body
    let body_bytes = body.collect().await
        .map(|c| c.to_bytes().to_vec())
        .unwrap_or_default();

    // Dispatch to a blocking thread for the synchronous middleware chain
    let http_response = tokio::task::spawn_blocking(move || {
        let nickel_req = request::Request::from_internal(
            method,
            uri,
            headers,
            body_bytes,
            &server.shared_data,
        );

        let nickel_res = response::Response::from_internal(
            &server.templates,
            &server.shared_data,
        );

        let final_res = server.middleware_stack.invoke(nickel_req, nickel_res);
        final_res.into_http_response()
    }).await.unwrap_or_else(|e| {
        eprintln!("Handler panicked: {}", e);
        http::Response::builder()
            .status(http::StatusCode::INTERNAL_SERVER_ERROR)
            .body(Vec::from("Internal Server Error"))
            .unwrap()
    });

    // Convert Vec<u8> body to Full<Bytes>
    let (parts, body) = http_response.into_parts();
    let response = hyper::Response::from_parts(parts, http_body_util::Full::new(bytes::Bytes::from(body)));

    Ok(response)
}

/// A server listening on a socket
pub struct ListeningServer {
    socket: SocketAddr,
    shutdown: Option<oneshot::Sender<()>>,
    handle: Option<std::thread::JoinHandle<()>>,
}

impl ListeningServer {
    /// Gets the `SocketAddr` which the server is currently listening on.
    pub fn socket(&self) -> SocketAddr {
        self.socket
    }

    /// Detaches the server thread.
    ///
    /// This doesn't actually kill the server, it just stops the current thread from
    /// blocking due to the server running. In the case where `main` returns due to
    /// this unblocking, then the server will be killed due to process death.
    ///
    /// The required use of this is when writing unit tests which spawn servers and do
    /// not want to block the test-runner by waiting on the server to stop because
    /// it probably never will.
    pub fn detach(mut self) {
        // Leak the shutdown sender so the server thread keeps running
        if let Some(tx) = self.shutdown.take() {
            std::mem::forget(tx);
        }
        // Take the handle out so Drop doesn't join (which would block)
        self.handle.take();
        // The server thread continues running. When the process exits it will be killed.
    }
}

impl Drop for ListeningServer {
    fn drop(&mut self) {
        // If we still have the thread handle, join it.
        // The server thread runs forever (accepting connections), so this blocks
        // until the process is killed — which is the expected behavior for a
        // server started with `server.listen("addr").unwrap()`.
        //
        // If `detach()` was called, handle is None and this is a no-op.
        //
        // Important: We must keep the shutdown sender alive (leak it) so that
        // the server thread's `select!` on `shutdown_rx` doesn't resolve.
        // If we just let `shutdown` drop, it would signal the server to stop.
        if let Some(tx) = self.shutdown.take() {
            std::mem::forget(tx);
        }
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

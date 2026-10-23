//! Run with `cargo run --example login_async`. The login protocol over tokio TCP.
use tpt_pulse_backend_tokio::{accept, connect, AsyncChan};
use tokio::net::TcpListener;
use tpt_pulse::prelude::*;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct AuthReq {
    user: String,
}
#[derive(Serialize, Deserialize)]
struct AuthOk(String);
#[derive(Serialize, Deserialize)]
struct AuthErr(String);
type LoginProto = Send<AuthReq, Either<Recv<AuthOk>, Recv<AuthErr>>>;

async fn login_client(chan: AsyncChan<LoginProto, Client>, user: &str) -> Result<String, String> {
    match chan.send(AuthReq { user: user.into() }).await.offer().await {
        Either::Left(chan) => {
            let (AuthOk(message), chan) = chan.recv().await;
            chan.close();
            Ok(message)
        }
        Either::Right(chan) => {
            let (AuthErr(message), chan) = chan.recv().await;
            chan.close();
            Err(message)
        }
    }
}

async fn login_server(chan: AsyncChan<<LoginProto as Dual>::Dual, Server>) {
    let (request, chan) = chan.recv().await;
    if request.user == "admin" {
        chan.choose_left()
            .await
            .send(AuthOk("Welcome, admin".into()))
            .await
            .close();
    } else {
        chan.choose_right()
            .await
            .send(AuthErr("Unknown user".into()))
            .await
            .close();
    }
}

#[tokio::main]
async fn main() {
    for user in ["admin", "guest"] {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("local address");
        let server = tokio::spawn(async move {
            let chan = accept::<LoginProto>(&listener).await.expect("accept");
            login_server(chan).await;
        });
        let client = connect::<LoginProto, _>(addr).await.expect("connect");
        println!("{user}: {:?}", login_client(client, user).await);
        server.await.expect("server task");
    }
}

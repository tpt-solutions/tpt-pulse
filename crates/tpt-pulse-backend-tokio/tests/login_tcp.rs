//! The login protocol over real tokio TCP sockets, for both offered branches.

use tpt_pulse_backend_tokio::{accept, connect, AsyncChan};
use tpt_pulse_core::{Client, Dual, Either, Recv, Send, Server};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;

#[derive(Serialize, Deserialize)]
struct AuthReq {
    user: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct AuthOk {
    welcome: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct AuthErr {
    reason: String,
}

type LoginProto = Send<AuthReq, Either<Recv<AuthOk>, Recv<AuthErr>>>;

async fn login_client(chan: AsyncChan<LoginProto, Client>, user: &str) -> Result<AuthOk, AuthErr> {
    match chan.send(AuthReq { user: user.into() }).await.offer().await {
        Either::Left(chan) => {
            let (ok, chan) = chan.recv().await;
            chan.close();
            Ok(ok)
        }
        Either::Right(chan) => {
            let (err, chan) = chan.recv().await;
            chan.close();
            Err(err)
        }
    }
}

async fn login_server(chan: AsyncChan<<LoginProto as Dual>::Dual, Server>) {
    let (request, chan) = chan.recv().await;
    if request.user == "admin" {
        chan.choose_left()
            .await
            .send(AuthOk {
                welcome: "Welcome, admin".into(),
            })
            .await
            .close();
    } else {
        chan.choose_right()
            .await
            .send(AuthErr {
                reason: "Unknown user".into(),
            })
            .await
            .close();
    }
}

#[tokio::test]
async fn login_success_and_failure_over_tcp() {
    for user in ["admin", "guest"] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let chan = accept::<LoginProto>(&listener).await.unwrap();
            login_server(chan).await;
        });
        let client = connect::<LoginProto, _>(addr).await.unwrap();
        let result = login_client(client, user).await;
        server.await.unwrap();
        match user {
            "admin" => assert_eq!(
                result,
                Ok(AuthOk {
                    welcome: "Welcome, admin".into()
                })
            ),
            _ => assert_eq!(
                result,
                Err(AuthErr {
                    reason: "Unknown user".into()
                })
            ),
        }
    }
}

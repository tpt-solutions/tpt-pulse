//! End-to-end login protocol coverage for both offered branches.

use pulse_core::{session_channel, Chan, Client, Dual, Either, Recv, Send, Server};

struct AuthReq {
    user: String,
}
#[derive(Debug, PartialEq)]
struct AuthOk {
    welcome: String,
}
#[derive(Debug, PartialEq)]
struct AuthErr {
    reason: String,
}

type LoginProto = Send<AuthReq, Either<Recv<AuthOk>, Recv<AuthErr>>>;

fn login_client(chan: Chan<LoginProto, Client>, user: &str) -> Result<AuthOk, AuthErr> {
    match chan.send(AuthReq { user: user.into() }).offer() {
        Either::Left(chan) => {
            let (ok, chan) = chan.recv();
            chan.close();
            Ok(ok)
        }
        Either::Right(chan) => {
            let (err, chan) = chan.recv();
            chan.close();
            Err(err)
        }
    }
}

fn login_server(chan: Chan<<LoginProto as Dual>::Dual, Server>) {
    let (request, chan) = chan.recv();
    if request.user == "admin" {
        chan.choose_left()
            .send(AuthOk {
                welcome: "Welcome, admin".into(),
            })
            .close();
    } else {
        chan.choose_right()
            .send(AuthErr {
                reason: "Unknown user".into(),
            })
            .close();
    }
}

#[test]
fn login_success_and_failure() {
    for user in ["admin", "guest"] {
        let (client, server) = session_channel::<LoginProto>();
        let peer = std::thread::spawn(move || login_server(server));
        let result = login_client(client, user);
        peer.join().unwrap();
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

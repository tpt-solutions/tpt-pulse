//! Run with `cargo run --example login`. Demonstrates both protocol branches.
use tpt_pulse::prelude::*;

struct AuthReq {
    user: String,
}
struct AuthOk(String);
struct AuthErr(String);
type LoginProto = Send<AuthReq, Either<Recv<AuthOk>, Recv<AuthErr>>>;

fn login_client(chan: Chan<LoginProto, Client>, user: &str) -> Result<String, String> {
    match chan.send(AuthReq { user: user.into() }).offer() {
        Either::Left(chan) => {
            let (AuthOk(message), chan) = chan.recv();
            chan.close();
            Ok(message)
        }
        Either::Right(chan) => {
            let (AuthErr(message), chan) = chan.recv();
            chan.close();
            Err(message)
        }
    }
}

fn main() {
    for user in ["admin", "guest"] {
        let (client, server) = session_channel::<LoginProto>();
        let peer = std::thread::spawn(move || {
            let (request, server) = server.recv();
            if request.user == "admin" {
                server
                    .choose_left()
                    .send(AuthOk("Welcome, admin".into()))
                    .close();
            } else {
                server
                    .choose_right()
                    .send(AuthErr("Unknown user".into()))
                    .close();
            }
        });
        let result = login_client(client, user);
        peer.join().expect("server completed");
        assert_eq!(result.is_ok(), user == "admin");
        println!("{user}: {result:?}");
    }
}

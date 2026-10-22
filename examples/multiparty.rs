//! Run with `cargo run --example multiparty`. Declares a three-role protocol,
//! checks it at compile time, and shows that a hand-written deadlock is caught.
use tpt_pulse::prelude::*;

fn main() {
    // Checked at compile time: a malformed step is a compile error.
    let protocol = global_protocol! {
        Client -> Gateway : Request;
        Gateway -> Backend : Forward;
        Backend -> Gateway : Reply;
        Gateway -> Client : Answer;
    };
    println!("roles: {:?}", protocol.roles());
    let session = protocol.project().expect("protocol is well formed");
    for program in session.programs() {
        println!("{}: {:?}", program.role, program.actions);
    }
    session
        .verify()
        .expect("projected session is deadlock-free");
    println!("projected session verified deadlock-free");

    // Two roles that each wait for the other to speak first.
    let deadlocked = Session::new(vec![
        Program {
            role: "A".into(),
            actions: vec![
                Action::Recv {
                    from: "B".into(),
                    label: "Ping".into(),
                },
                Action::Send {
                    to: "B".into(),
                    label: "Pong".into(),
                },
            ],
        },
        Program {
            role: "B".into(),
            actions: vec![
                Action::Recv {
                    from: "A".into(),
                    label: "Pong".into(),
                },
                Action::Send {
                    to: "A".into(),
                    label: "Ping".into(),
                },
            ],
        },
    ]);
    match deadlocked.verify() {
        Err(error) => println!("hand-written session rejected: {error}"),
        Ok(()) => panic!("the deadlock was not detected"),
    }
}

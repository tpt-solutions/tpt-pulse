//! Recursive protocols: [`Rec`] loops and [`Var`] jumps back to the loop head.
//!
//! The loop head is the state `Rec<P>`. [`Chan::enter`] unfolds it into one pass
//! through the body `P`, in which every `Var` stands for `Rec<P>` again. A loop
//! therefore has the same endpoint type on every iteration, so it runs in an
//! ordinary Rust `loop`, and the exit is any branch that reaches [`Close`].

use std::marker::PhantomData;

use crate::{sealed, Chan, Choose, Close, Dual, Either, Protocol, Recv, Send};

/// A recursive protocol. Its body `P` may return to the loop head with [`Var`].
///
/// Like every protocol state, an endpoint at `Rec<P>` must eventually reach
/// [`Close`], so a loop needs an exit branch.
pub struct Rec<P>(PhantomData<fn() -> P>);

/// Returns to the nearest enclosing [`Rec`] loop head. Only meaningful in a loop body.
pub struct Var;

/// Replaces every [`Var`] in a protocol with `X`.
///
/// Implemented for every protocol state. Users do not implement it.
pub trait Subst<X> {
    /// The protocol with each `Var` replaced by `X`.
    type Out: Protocol;
}

impl<X: Protocol> Subst<X> for Var {
    type Out = X;
}

impl<X> Subst<X> for Close {
    type Out = Close;
}

impl<M: std::marker::Send + 'static, P: Subst<X>, X> Subst<X> for Send<M, P> {
    type Out = Send<M, P::Out>;
}

impl<M: std::marker::Send + 'static, P: Subst<X>, X> Subst<X> for Recv<M, P> {
    type Out = Recv<M, P::Out>;
}

impl<L: Subst<X>, R: Subst<X>, X> Subst<X> for Either<L, R> {
    type Out = Either<L::Out, R::Out>;
}

impl<L: Subst<X>, R: Subst<X>, X> Subst<X> for Choose<L, R> {
    type Out = Choose<L::Out, R::Out>;
}

/// A nested loop keeps its own `Var`s, so substitution stops at its head.
impl<P: Protocol, X> Subst<X> for Rec<P> {
    type Out = Rec<P>;
}

impl sealed::Sealed for Var {}
impl Protocol for Var {}
impl Dual for Var {
    type Dual = Var;
}

impl<P: Protocol> sealed::Sealed for Rec<P> {}
impl<P: Protocol> Protocol for Rec<P> {}
impl<P: Dual> Dual for Rec<P> {
    type Dual = Rec<P::Dual>;
}

impl<P: Protocol + Subst<Rec<P>>, R> Chan<Rec<P>, R> {
    /// Enter one pass through the loop body.
    ///
    /// Each `Var` in the body is `Rec<P>` again, so call `enter` at the top of
    /// every iteration.
    pub fn enter(mut self) -> Chan<<P as Subst<Rec<P>>>::Out, R> {
        Chan::new(self.take_transport())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{session_channel, Client, Server};

    struct Tick;
    struct Done;

    /// The client chooses to tick again or to finish.
    type Body = Choose<Send<Tick, Var>, Send<Done, Close>>;
    type Counter = Rec<Body>;
    type CounterPeer = Rec<Either<Recv<Tick, Var>, Recv<Done, Close>>>;

    fn client(mut chan: Chan<Counter, Client>, ticks: u32) {
        let mut sent = 0;
        loop {
            let body = chan.enter();
            if sent == ticks {
                body.choose_right().send(Done).close();
                return;
            }
            chan = body.choose_left().send(Tick);
            sent += 1;
        }
    }

    fn server(mut chan: Chan<CounterPeer, Server>) -> u32 {
        let mut received = 0;
        loop {
            match chan.enter().offer() {
                Either::Left(tick) => {
                    let (Tick, next) = tick.recv();
                    received += 1;
                    chan = next;
                }
                Either::Right(done) => {
                    let (Done, end) = done.recv();
                    end.close();
                    return received;
                }
            }
        }
    }

    #[test]
    fn a_loop_runs_until_its_exit_branch() {
        let (client_end, server_end) = session_channel::<Counter>();
        let peer = std::thread::spawn(move || server(server_end));
        client(client_end, 3);
        assert_eq!(peer.join().unwrap(), 3);
    }

    #[test]
    fn a_loop_can_exit_immediately() {
        let (client_end, server_end) = session_channel::<Counter>();
        let peer = std::thread::spawn(move || server(server_end));
        client(client_end, 0);
        assert_eq!(peer.join().unwrap(), 0);
    }
}

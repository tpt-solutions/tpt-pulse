use crate::{sealed, Chan, Dual, PhantomData, Protocol};

/// An offered protocol branch, or the selected branch's endpoint at runtime.
///
/// As a protocol, `Either<L, R>` lets the peer select which continuation to use.
/// [`Chan::offer`] returns this enum containing the chosen endpoint.
#[derive(Debug, PartialEq, Eq)]
pub enum Either<L, R> {
    /// Left branch.
    Left(L),
    /// Right branch.
    Right(R),
}

/// Select one of two protocol continuations; dual to [`Either`].
pub struct Choose<L, R>(PhantomData<fn() -> (L, R)>);

impl<L: Protocol, R: Protocol> sealed::Sealed for Either<L, R> {}
impl<L: Protocol, R: Protocol> Protocol for Either<L, R> {}
impl<L: Protocol, R: Protocol> sealed::Sealed for Choose<L, R> {}
impl<L: Protocol, R: Protocol> Protocol for Choose<L, R> {}
impl<L: Dual, R: Dual> Dual for Either<L, R> {
    type Dual = Choose<L::Dual, R::Dual>;
}
impl<L: Dual, R: Dual> Dual for Choose<L, R> {
    type Dual = Either<L::Dual, R::Dual>;
}

// A private wire discriminator cannot be confused with an application message.
enum Selection {
    Left,
    Right,
}

impl<L: Protocol, R: Protocol, Role> Chan<Choose<L, R>, Role> {
    /// Select the left continuation. Panics if the peer disconnected.
    pub fn choose_left(mut self) -> Chan<L, Role> {
        let transport = self.take_transport();
        assert!(
            transport.tx.send(Box::new(Selection::Left)).is_ok(),
            "session peer disconnected during choice"
        );
        Chan::new(transport)
    }

    /// Select the right continuation. Panics if the peer disconnected.
    pub fn choose_right(mut self) -> Chan<R, Role> {
        let transport = self.take_transport();
        assert!(
            transport.tx.send(Box::new(Selection::Right)).is_ok(),
            "session peer disconnected during choice"
        );
        Chan::new(transport)
    }
}

impl<L: Protocol, R: Protocol, Role> Chan<Either<L, R>, Role> {
    /// Wait for the peer's selection and return its typed continuation.
    /// Panics if the peer disconnected.
    pub fn offer(mut self) -> Either<Chan<L, Role>, Chan<R, Role>> {
        let transport = self.take_transport();
        let selection = transport
            .rx
            .recv()
            .expect("session peer disconnected during offer");
        match *selection
            .downcast::<Selection>()
            .unwrap_or_else(|_| panic!("internal session branch mismatch"))
        {
            Selection::Left => Either::Left(Chan::new(transport)),
            Selection::Right => Either::Right(Chan::new(transport)),
        }
    }
}

use std::sync::PoisonError;
use tonic::Status;

pub struct LockPoisoned;

impl<T> From<PoisonError<T>> for LockPoisoned {
    fn from(_: PoisonError<T>) -> Self {
        LockPoisoned
    }
}

impl From<LockPoisoned> for Status {
    fn from(_: LockPoisoned) -> Self {
        Status::internal("user store lock poisoned")
    }
}

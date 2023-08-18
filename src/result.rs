use std::ops::{Deref, DerefMut};

use crate::error::Error;

pub(crate) type InternalResult<T> = std::result::Result<T, Error>;

pub struct Result<T>(InternalResult<T>);

impl<T> Result<T> {
    pub fn into_inner(self) -> InternalResult<T> {
        self.0
    }
}

impl<T> From<InternalResult<T>> for Result<T> {
    fn from(value: InternalResult<T>) -> Self {
        Result(value)
    }
}

impl<T> From<Result<T>> for InternalResult<T> {
    fn from(value: Result<T>) -> Self {
        value.into_inner()
    }
}

impl<T> AsRef<InternalResult<T>> for Result<T> {
    fn as_ref(&self) -> &InternalResult<T> {
        &self.0
    }
}

impl<T> AsMut<InternalResult<T>> for Result<T> {
    fn as_mut(&mut self) -> &mut InternalResult<T> {
        &mut self.0
    }
}

impl<T> Deref for Result<T> {
    type Target = InternalResult<T>;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<T> DerefMut for Result<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_into_inner() {
        let result = Result(Ok("Str"));

        assert!(result
            .into_inner()
            .unwrap()
            .starts_with("Str"));
    }
}

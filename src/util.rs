use std::panic::Location;
use anyhow::Context;

pub trait AddSpan<T> {
    fn add_span(self) -> anyhow::Result<T>;
}

impl<T, E> AddSpan<T> for Result<T, E> where E: Into<anyhow::Error> {
    #[track_caller]
    fn add_span(self) -> anyhow::Result<T> {
        let loc = Location::caller();
        self.map_err(|err| err.into()).with_context(|| format!("at {}:{}:{}", loc.file(), loc.line(), loc.column()))
    }
}
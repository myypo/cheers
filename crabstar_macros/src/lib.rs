mod crabstar;

pub use crabstar::args::{CrabstarArgs, CrabstarPageArgs, CrabstarSuspenseArgs};
pub use crabstar::{crabstar_derive, inject_scripts};
use proc_macro2::Span;
use std::fmt::Display;
use std::sync::Arc;

#[derive(Clone)]
pub enum Source {
    Path(Arc<str>),
    Source(Arc<str>),
}

#[derive(Debug, Clone)]
pub struct CompileError {
    pub msg: String,
    pub span: Option<Span>,
}

impl CompileError {
    pub fn new<S: ToString>(msg: S, span: Option<Span>) -> Self {
        Self {
            msg: msg.to_string(),
            span,
        }
    }
}

impl std::error::Error for CompileError {}

impl Display for CompileError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.write_str(&self.msg)
    }
}

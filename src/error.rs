use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub(crate) struct NoPathParent;

impl Display for NoPathParent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid path parent")
    }
}

impl Error for NoPathParent {}

#[derive(Debug)]
pub(crate) struct NoFileName;

impl Display for NoFileName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid file name")
    }
}

impl Error for NoFileName {}
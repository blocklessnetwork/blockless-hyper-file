use std::fmt::{
    Display, 
    Formatter, 
    self
};

#[derive(Debug)]
pub enum ParseError {
    // the invalid range
    InvalidRange,
    //NoOverlap
    NoOverlap,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            ParseError::InvalidRange => write!(f, "invaild range"),
            ParseError::NoOverlap => write!(f, "no overlap"),
        }
    }
}
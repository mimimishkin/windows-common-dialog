use std::fmt::{Display, Formatter};

#[derive(Debug, Clone)]
pub struct MacDialogError(pub String);

impl Display for MacDialogError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error in chooser dialog: {}", self.0)
    }
}

pub type MacRes<T> = Result<T, MacDialogError>;
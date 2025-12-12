pub enum RequestError {
    FailedToParse,
    UnknownCommand,
}

impl From<Option<&str>> for RequestError {
    fn from(_value: Option<&str>) -> Self {
        RequestError::FailedToParse
    }
}

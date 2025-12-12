use crate::utilites::error::{self, RequestError};

pub struct ParsedRequest<'a> {
    pub command: &'a str,
    pub args: Vec<&'a str>,
}

pub fn select_request(request: &str) -> Result<ParsedRequest<'_>, RequestError> {
    let mut parts = request.trim_ascii().split_whitespace();

    let command = parts.next().ok_or(RequestError::FailedToParse)?;

    let args = parts.collect();

    Ok(ParsedRequest { command, args })
}

pub fn request_router(request: &str) -> Result<(), RequestError> {
    let parsed = select_request(request)?;

    match parsed.command {
        "LOOKUP" => Ok(()),
        "PING" => Ok(()),
        "UPDATE" => Ok(()),
        "CREATE" => Ok(()),

        _ => Err(RequestError::UnknownCommand),
    }
}

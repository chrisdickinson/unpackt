error_chain! {
    foreign_links {
        Io(::std::io::Error);
        Encoding(::std::str::Utf8Error);
        BadMode(::std::num::ParseIntError);
    }

    errors {
        NotFound
        Request
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        ErrorKind::Request.into()
    }
}

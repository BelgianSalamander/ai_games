use std::{collections::HashMap, fmt, str::FromStr, string::FromUtf8Error};

use crate::util::asyncio::{AsyncReaderWrapper, AsyncWriterWrapper};

use super::web_errors::WebError;

#[derive(Debug, Copy, Clone)]
pub enum Status {
    Continue,           //100
    SwitchingProtocols, //101

    Ok,                          //200
    Created,                     //201
    Accepted,                    //202
    NonAuthoritativeInformation, //203
    NoContent,                   //204
    ResetContent,                //205
    PartialContent,              //206

    MultipleChoices,   //300
    MovedPermanently,  //301
    Found,             //302
    SeeOther,          //303
    NotModified,       //304
    UseProxy,          //305
    TemporaryRedirect, //307
    PermanentRedirect, //308

    BadRequest,                  //400
    Unauthorized,                //401
    PaymentRequired,             //402
    Forbidden,                   //403
    NotFound,                    //404
    MethodNotAllowed,            //405
    NotAcceptable,               //406
    ProxyAuthenticationRequired, //407
    RequestTimeout,              //408
    Conflict,                    //409
    Gone,                        //410
    LengthRequired,              //411
    PreconditionFailed,          //412
    PayloadTooLarge,             //413
    UriTooLong,                  //414
    UnsupportedMediaType,        //415
    RangeNotSatisfiable,         //416
    ExpectationFailed,           //417
    ImATeapot,                   //418
    MisdirectedRequest,          //421
    UnprocessableEntity,         //422
    Locked,                      //423
    FailedDependency,            //424
    UpgradeRequired,             //426
    PreconditionRequired,        //428
    TooManyRequests,             //429
    RequestHeaderFieldsTooLarge, //431
    UnavailableForLegalReasons,  //451

    InternalServerError,           //500
    NotImplemented,                //501
    BadGateway,                    //502
    ServiceUnavailable,            //503
    GatewayTimeout,                //504
    HttpVersionNotSupported,       //505
    VariantAlsoNegotiates,         //506
    InsufficientStorage,           //507
    LoopDetected,                  //508
    NotExtended,                   //510
    NetworkAuthenticationRequired, //511

    Other(u16, &'static str),
}

impl Status {
    pub fn get_code(&self) -> u16 {
        match self {
            Status::Continue => 100,
            Status::SwitchingProtocols => 101,

            Status::Ok => 200,
            Status::Created => 201,
            Status::Accepted => 202,
            Status::NonAuthoritativeInformation => 203,
            Status::NoContent => 204,
            Status::ResetContent => 205,
            Status::PartialContent => 206,

            Status::MultipleChoices => 300,
            Status::MovedPermanently => 301,
            Status::Found => 302,
            Status::SeeOther => 303,
            Status::NotModified => 304,
            Status::UseProxy => 305,
            Status::TemporaryRedirect => 307,
            Status::PermanentRedirect => 308,

            Status::BadRequest => 400,
            Status::Unauthorized => 401,
            Status::PaymentRequired => 402,
            Status::Forbidden => 403,
            Status::NotFound => 404,
            Status::MethodNotAllowed => 405,
            Status::NotAcceptable => 406,
            Status::ProxyAuthenticationRequired => 407,
            Status::RequestTimeout => 408,
            Status::Conflict => 409,
            Status::Gone => 410,
            Status::LengthRequired => 411,
            Status::PreconditionFailed => 412,
            Status::PayloadTooLarge => 413,
            Status::UriTooLong => 414,
            Status::UnsupportedMediaType => 415,
            Status::RangeNotSatisfiable => 416,
            Status::ExpectationFailed => 417,
            Status::ImATeapot => 418,
            Status::MisdirectedRequest => 421,
            Status::UnprocessableEntity => 422,
            Status::Locked => 423,
            Status::FailedDependency => 424,
            Status::UpgradeRequired => 426,
            Status::PreconditionRequired => 428,
            Status::TooManyRequests => 429,
            Status::RequestHeaderFieldsTooLarge => 431,
            Status::UnavailableForLegalReasons => 451,

            Status::InternalServerError => 500,
            Status::NotImplemented => 501,
            Status::BadGateway => 502,
            Status::ServiceUnavailable => 503,
            Status::GatewayTimeout => 504,
            Status::HttpVersionNotSupported => 505,
            Status::VariantAlsoNegotiates => 506,
            Status::InsufficientStorage => 507,
            Status::LoopDetected => 508,
            Status::NotExtended => 510,
            Status::NetworkAuthenticationRequired => 511,

            Status::Other(code, _) => *code,
        }
    }

    pub fn get_message(&self) -> &'static str {
        match self {
            Status::Continue => "Continue",
            Status::SwitchingProtocols => "Switching Protocols",

            Status::Ok => "OK",
            Status::Created => "Created",
            Status::Accepted => "Accepted",
            Status::NonAuthoritativeInformation => "Non-Authoritative Information",
            Status::NoContent => "No Content",
            Status::ResetContent => "Reset Content",
            Status::PartialContent => "Partial Content",

            Status::MultipleChoices => "Multiple Choices",
            Status::MovedPermanently => "Moved Permanently",
            Status::Found => "Found",
            Status::SeeOther => "See Other",
            Status::NotModified => "Not Modified",
            Status::UseProxy => "Use Proxy",
            Status::TemporaryRedirect => "Temporary Redirect",
            Status::PermanentRedirect => "Permanent Redirect",

            Status::BadRequest => "Bad Request",
            Status::Unauthorized => "Unauthorized",
            Status::PaymentRequired => "Payment Required",
            Status::Forbidden => "Forbidden",
            Status::NotFound => "Not Found",
            Status::MethodNotAllowed => "Method Not Allowed",
            Status::NotAcceptable => "Not Acceptable",
            Status::ProxyAuthenticationRequired => "Proxy Authentication Required",
            Status::RequestTimeout => "Request Timeout",
            Status::Conflict => "Conflict",
            Status::Gone => "Gone",
            Status::LengthRequired => "Length Required",
            Status::PreconditionFailed => "Precondition Failed",
            Status::PayloadTooLarge => "Payload Too Large",
            Status::UriTooLong => "URI Too Long",
            Status::UnsupportedMediaType => "Unsupported Media Type",
            Status::RangeNotSatisfiable => "Range Not Satisfiable",
            Status::ExpectationFailed => "Expectation Failed",
            Status::ImATeapot => "I'm a teapot",
            Status::MisdirectedRequest => "Misdirected Request",
            Status::UnprocessableEntity => "Unprocessable Entity",
            Status::Locked => "Locked",
            Status::FailedDependency => "Failed Dependency",
            Status::UpgradeRequired => "Upgrade Required",
            Status::PreconditionRequired => "Precondition Required",
            Status::TooManyRequests => "Too Many Requests",
            Status::RequestHeaderFieldsTooLarge => "Request Header Fields Too Large",
            Status::UnavailableForLegalReasons => "Unavailable For Legal Reasons",

            Status::InternalServerError => "Internal Server Error",
            Status::NotImplemented => "Not Implemented",
            Status::BadGateway => "Bad Gateway",
            Status::ServiceUnavailable => "Service Unavailable",
            Status::GatewayTimeout => "Gateway Timeout",
            Status::HttpVersionNotSupported => "HTTP Version Not Supported",
            Status::VariantAlsoNegotiates => "Variant Also Negotiates",
            Status::InsufficientStorage => "Insufficient Storage",
            Status::LoopDetected => "Loop Detected",
            Status::NotExtended => "Not Extended",
            Status::NetworkAuthenticationRequired => "Network Authentication Required",

            Status::Other(_, message) => message,
        }
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} {}", self.get_code(), self.get_message())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Trace,
    Connect,
    Patch,
    Other(String),
}

impl Method {
    pub fn get_name(&self) -> &str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Head => "HEAD",
            Method::Options => "OPTIONS",
            Method::Trace => "TRACE",
            Method::Connect => "CONNECT",
            Method::Patch => "PATCH",
            Method::Other(name) => name,
        }
    }
}

impl fmt::Display for Method {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_name())
    }
}

#[derive(Debug)]
pub enum HttpError {
    Http(Status, Option<String>),
    Io(std::io::Error),
    Other(String),
}

impl From<std::io::Error> for HttpError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<FromUtf8Error> for HttpError {
    fn from(err: FromUtf8Error) -> Self {
        Self::Other(format!("Failed to decode a string. All strings should be sent as UTF-8 or ASCII. {:?}", err))
    }
}

impl From<HttpError> for std::io::Error {
    fn from(err: HttpError) -> Self {
        match err {
            HttpError::Io(err) => err,
            HttpError::Http(status, Some(message)) => std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("{}: {}", status, message),
            ),
            HttpError::Http(status, None) => {
                std::io::Error::new(std::io::ErrorKind::Other, format!("{}", status))
            }
            HttpError::Other(message) => std::io::Error::new(std::io::ErrorKind::Other, message),
        }
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HttpError::Http(status, message) => {
                if let Some(message) = message {
                    write!(f, "{}: {}", status, message)
                } else {
                    write!(f, "{}", status)
                }
            }
            HttpError::Io(err) => write!(f, "{}", err),
            HttpError::Other(message) => write!(f, "{}", message),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RequestPath {
    pub path: Vec<String>,
    pub query: HashMap<String, String>,
}

impl fmt::Display for RequestPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.path.join("/"))?;

        if self.query.len() > 0 {
            write!(
                f,
                "?{}",
                self.query
                    .iter()
                    .map(|(key, value)| format!("{}={}", key, value))
                    .collect::<Vec<_>>()
                    .join("&")
            )?;
        }

        Ok(())
    }
}

impl RequestPath {
    pub fn new(path: Vec<String>, query: HashMap<String, String>) -> Self {
        Self { path, query }
    }

    pub fn parse(path: &str) -> Result<Self, HttpError> {
        let mut path_parts = path.split('?');

        let path = path_parts
            .next()
            .unwrap()
            .split('/')
            .map(|x| x.trim().to_string())
            .filter(|x| !x.is_empty())
            .collect::<Vec<_>>();

        let mut query = HashMap::new();

        if let Some(query_string) = path_parts.next() {
            for query_param in query_string.split('&') {
                let mut query_parts = query_param.split('=');

                let key = match query_parts.next() {
                    Some(key) => key,
                    None => {
                        return Err(HttpError::Http(
                            Status::BadRequest,
                            Some("Invalid query string".to_string()),
                        ))
                    }
                };
                let value = match query_parts.next() {
                    Some(value) => value,
                    None => {
                        return Err(HttpError::Http(
                            Status::BadRequest,
                            Some("Invalid query string".to_string()),
                        ))
                    }
                };

                query.insert(key.to_string(), value.to_string());
            }
        }

        Ok(Self::new(path, query))
    }

    pub fn parse_query<T: FromStr>(&self, key: &str) -> Result<T, WebError> {
        match self.get(key)?.parse() {
            Ok(x) => Ok(x),
            Err(_) => Err(WebError::InvalidData(format!("Couldn't parse parameter '{}'", key))),
        }
    }

    pub fn get(&self, key: &str) -> Result<&String, WebError> {
        match self.query.get(key) {
            Some(s) => Ok(s),
            None => Err(WebError::MissingParameter(format!("'{}'", key))),
        }
    }
}

#[derive(Debug, Clone)]
struct HttpMessage {
    first_line: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

impl HttpMessage {
    pub fn new(first_line: String, headers: HashMap<String, String>, body: Vec<u8>) -> Self {
        Self {
            first_line,
            headers,
            body,
        }
    }

    pub async fn parse_async<T: async_std::io::Read + Unpin>(
        stream: &mut T,
    ) -> Result<Self, HttpError> {
        let mut stream = AsyncReaderWrapper::new(stream);

        let mut buf = [0u8; 4096];
        let mut buf_pos = 0;
        let mut buf_end = 0;

        let mut lines = Vec::new();

        let mut curr_line = String::new();

        loop {
            if buf_pos == buf_end {
                buf_pos = 0;
                buf_end = stream.read(&mut buf).await?;
            }

            if buf_end == 0 {
                return Err(HttpError::Http(
                    Status::BadRequest,
                    Some("Empty request".to_string()),
                ));
            }

            let c = buf[buf_pos];
            buf_pos += 1;

            if c == b'\r' {
                continue;
            }

            if c == b'\n' {
                if curr_line.is_empty() {
                    break;
                }

                lines.push(curr_line);
                curr_line = String::new();
            } else {
                curr_line.push(c as char);
            }
        }

        let mut headers = HashMap::new();

        for line in lines.iter().skip(1) {
            let colon_idx = line.find(':').ok_or(HttpError::Http(
                Status::BadRequest,
                Some("Invalid header".to_string()),
            ))?;

            headers.insert(
                line[..colon_idx].to_string(),
                line[colon_idx + 1..].trim().to_string(),
            );
        }

        let mut body = Vec::new();

        if let Some(content_length) = headers.get("Content-Length") {
            let content_length = content_length.parse::<usize>().map_err(|_| {
                HttpError::Http(
                    Status::BadRequest,
                    Some("Invalid Content-Length".to_string()),
                )
            })?;

            body.resize(content_length, 0);

            //Read remaining from buffer
            let body_in_buf = (buf_end - buf_pos).min(content_length);

            body[..body_in_buf].copy_from_slice(&buf[buf_pos..buf_pos + body_in_buf]);

            //Read remaining from stream
            stream.read_exact(&mut body[body_in_buf..]).await?;
        }

        Ok(Self::new(lines[0].to_string(), headers, body))
    }
}

#[derive(Debug, Clone)]
pub struct Request {
    pub method: Method,
    pub path: RequestPath,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub cookies: HashMap<String, String>,
}

impl Request {
    pub fn new(
        method: Method,
        path: RequestPath,
        headers: HashMap<String, String>,
        body: Vec<u8>,
        cookies: HashMap<String, String>,
    ) -> Self {
        Self {
            method,
            path,
            headers,
            body,
            cookies,
        }
    }

    pub async fn parse_async<T: async_std::io::Read + Unpin>(
        stream: &mut T,
    ) -> Result<Self, HttpError> {
        let message = HttpMessage::parse_async(stream).await?;

        let request_line = message.first_line.split(' ').collect::<Vec<_>>();

        if request_line.len() != 3 {
            return Err(HttpError::Http(
                Status::BadRequest,
                Some("Invalid request line".to_string()),
            ));
        }

        let method = match request_line[0] {
            "GET" => Method::Get,
            "POST" => Method::Post,
            "PUT" => Method::Put,
            "DELETE" => Method::Delete,
            "HEAD" => Method::Head,
            "OPTIONS" => Method::Options,
            "TRACE" => Method::Trace,
            "CONNECT" => Method::Connect,
            "PATCH" => Method::Patch,
            _ => Method::Other(request_line[0].to_string()),
        };

        let path = RequestPath::parse(request_line[1])?;

        let cookies = message
            .headers
            .get("Cookie")
            .map(|cookie_header| {
                let mut cookies = HashMap::new();

                for cookie in cookie_header.split(';') {
                    let mut cookie_parts = cookie.split('=');

                    let key = match cookie_parts.next() {
                        Some(key) => key,
                        None => continue,
                    };
                    let value = match cookie_parts.next() {
                        Some(value) => value,
                        None => continue,
                    };

                    cookies.insert(key.trim().to_string(), value.trim().to_string());
                }

                cookies
            })
            .unwrap_or(HashMap::new());

        Ok(Self::new(
            method,
            path,
            message.headers,
            message.body,
            cookies,
        ))
    }

    pub async fn write_async<T: async_std::io::Write + Unpin>(
        self,
        stream: &mut T,
    ) -> Result<(), HttpError> {
        let mut stream = AsyncWriterWrapper::new(stream);

        stream
            .write(format!("{} {} HTTP/1.1", self.method, self.path.path.join("/")).as_bytes())
            .await?;

        if self.path.query.len() > 0 {
            stream
                .write(
                    format!(
                        "?{}",
                        self.path
                            .query
                            .iter()
                            .map(|(key, value)| format!("{}={}", key, value))
                            .collect::<Vec<_>>()
                            .join("&")
                    )
                    .as_bytes(),
                )
                .await?;
        }

        stream.write(b"\r\n").await?;

        for (key, value) in self.headers.iter() {
            stream
                .write(format!("{}: {}\r\n", key, value).as_bytes())
                .await?;
        }

        stream.write(b"\r\n").await?;

        stream.write(&self.body).await?;

        Ok(())
    }

    pub fn matches_path(&self, path: &[&str]) -> bool {
        if path.len() > self.path.path.len() {
            return false;
        }

        for (i, path_part) in path.iter().enumerate() {
            if path_part != &self.path.path[i].as_str() {
                return false;
            }
        }

        true
    }

    pub fn matches_path_exact(&self, path: &[&str]) -> bool {
        if path.len() != self.path.path.len() {
            return false;
        }

        for (i, path_part) in path.iter().enumerate() {
            if path_part != &self.path.path[i].as_str() {
                return false;
            }
        }

        true
    }

    pub fn get_cookies(&self) -> HashMap<String, String> {
        let mut cookies = HashMap::new();

        if let Some(cookie_header) = self.headers.get("Cookie") {
            for cookie in cookie_header.split(';') {
                let mut cookie_parts = cookie.split('=');

                let key = match cookie_parts.next() {
                    Some(key) => key,
                    None => continue,
                };
                let value = match cookie_parts.next() {
                    Some(value) => value,
                    None => continue,
                };

                cookies.insert(key.trim().to_string(), value.trim().to_string());
            }
        }

        cookies
    }
}

#[derive(Debug, Clone)]
pub struct Response {
    pub status: Status,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new() -> Self {
        Self {
            status: Status::Ok,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn basic_error(status: Status, message: &str) -> Self {
        let mut response = Self::new();

        response.set_status(status);
        response.set_header("Content-Type", "text/plain");
        response.set_body(message.as_bytes().to_vec());

        response
    }

    pub fn ok() -> Self {
        let mut res = Response::new();
        res.set_status(Status::Ok);

        res
    }

    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }

    pub fn set_header(&mut self, key: &str, value: &str) {
        self.headers.insert(key.to_string(), value.to_string());
    }

    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body = body;

        self.set_header("Content-Length", &self.body.len().to_string());
    }

    pub async fn write_async<T: async_std::io::Write + Unpin>(
        self,
        stream: &mut T,
    ) -> Result<(), HttpError> {
        let mut stream = AsyncWriterWrapper::new(stream);

        stream
            .write_all(
                format!(
                    "HTTP/1.1 {} {}\r\n",
                    self.status.get_code(),
                    self.status.get_message()
                )
                .as_bytes(),
            )
            .await?;

        for (key, value) in self.headers.iter() {
            stream
                .write_all(format!("{}: {}\r\n", key, value).as_bytes())
                .await?;
        }

        stream.write_all(b"\r\n").await?;

        stream.write_all(&self.body).await?;

        Ok(())
    }

    pub async fn parse_async<T: async_std::io::Read + Unpin>(
        stream: &mut T,
    ) -> Result<Self, HttpError> {
        let message = HttpMessage::parse_async(stream).await?;

        let status_line = message
            .first_line
            .split(' ')
            .map(|x| x.trim().to_string())
            .collect::<Vec<_>>();

        if status_line.len() != 3 {
            return Err(HttpError::Http(
                Status::BadRequest,
                Some("Invalid status line".to_string()),
            ));
        }

        let status = &status_line[1];
        let status = u16::from_str_radix(&status, 10).map_err(|_| {
            HttpError::Http(
                Status::BadRequest,
                Some("Status code is not a u16".to_string()),
            )
        })?;

        let status = match status {
            100 => Status::Continue,
            101 => Status::SwitchingProtocols,

            200 => Status::Ok,
            201 => Status::Created,
            202 => Status::Accepted,
            203 => Status::NonAuthoritativeInformation,
            204 => Status::NoContent,
            205 => Status::ResetContent,
            206 => Status::PartialContent,

            300 => Status::MultipleChoices,
            301 => Status::MovedPermanently,
            302 => Status::Found,
            303 => Status::SeeOther,
            304 => Status::NotModified,
            305 => Status::UseProxy,
            307 => Status::TemporaryRedirect,
            308 => Status::PermanentRedirect,

            400 => Status::BadRequest,
            401 => Status::Unauthorized,
            402 => Status::PaymentRequired,
            403 => Status::Forbidden,
            404 => Status::NotFound,
            405 => Status::MethodNotAllowed,
            406 => Status::NotAcceptable,
            407 => Status::ProxyAuthenticationRequired,
            408 => Status::RequestTimeout,
            409 => Status::Conflict,
            410 => Status::Gone,
            411 => Status::LengthRequired,
            412 => Status::PreconditionFailed,
            413 => Status::PayloadTooLarge,
            414 => Status::UriTooLong,
            415 => Status::UnsupportedMediaType,
            416 => Status::RangeNotSatisfiable,
            417 => Status::ExpectationFailed,
            418 => Status::ImATeapot,
            421 => Status::MisdirectedRequest,
            422 => Status::UnprocessableEntity,
            423 => Status::Locked,
            424 => Status::FailedDependency,
            426 => Status::UpgradeRequired,
            428 => Status::PreconditionRequired,
            429 => Status::TooManyRequests,
            431 => Status::RequestHeaderFieldsTooLarge,
            451 => Status::UnavailableForLegalReasons,

            500 => Status::InternalServerError,
            501 => Status::NotImplemented,
            502 => Status::BadGateway,
            503 => Status::ServiceUnavailable,
            504 => Status::GatewayTimeout,
            505 => Status::HttpVersionNotSupported,
            506 => Status::VariantAlsoNegotiates,
            507 => Status::InsufficientStorage,
            508 => Status::LoopDetected,
            510 => Status::NotExtended,
            511 => Status::NetworkAuthenticationRequired,

            _ => {
                return Err(HttpError::Http(
                    Status::BadRequest,
                    Some("Invalid status code".to_string()),
                ))
            }
        };

        Ok(Self {
            status,
            headers: message.headers,
            body: message.body,
        })
    }
}

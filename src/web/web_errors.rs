
use std::{str::FromStr, error::Error, fmt::Debug};

use serde_json::{Value, Map, Number};

use super::http::{Response, Status};

#[derive(Debug, Clone)]
pub enum WebError {
    InvalidData(String),
    MissingParameter(String),
    NotFound(String),
    InvalidMethod,
    InternalServerError(String),
    Unauthorized
}

impl<T: Error + Debug> From<T> for WebError {
    fn from(value: T) -> Self {
        WebError::InternalServerError(format!("{:?}", value))
    }
}

impl WebError {
    pub fn into_response(self) -> Response {
        let mut response = Response::new();

        match self {
            Self::InvalidData(message) => {
                response.set_status(Status::BadRequest);
                response.set_header("Content-Type", "text/plain");
                response.set_body(format!("Invalid Data: {}", message).into_bytes());
            },
            Self::MissingParameter(parameter) => {
                response.set_status(Status::BadRequest);
                response.set_header("Content-Type", "text/plain");
                response.set_body(format!("Missing Parameter: {}", parameter).into_bytes());
            },
            Self::NotFound(message) => {
                response.set_status(Status::NotFound);
                response.set_header("Content-Type", "text/plain");
                response.set_body(format!("Not Found: {}", message).into_bytes());
            },
            Self::InvalidMethod => {
                response.set_status(Status::NotImplemented);
                response.set_header("Content-Type", "text/plain");
                response.set_body("HTTP Method not implemented".as_bytes().to_vec());
            },
            Self::InternalServerError(message) => {
                response.set_status(Status::InternalServerError);
                response.set_header("Content-Type", "text/plain");
                response.set_body(format!("Internal Server Error: {}", message).into_bytes());
            },
            Self::Unauthorized => {
                response.set_status(Status::Forbidden);
                response.set_header("Content-Type", "text/plain");
                response.set_body("Unauthorized".as_bytes().to_vec());
            }
        }

        response
    }
}

pub type HttpResult<T> = Result<T, WebError>;

fn get_kind_str(v: &Value) -> &'static str {
    match v {
        Value::Array(_) => "array",
        Value::Bool(_) => "bool",
        Value::Null => "null",
        Value::Number(_) => "number",
        Value::Object(_) => "object",
        Value::String(_) => "string"
    }
}

pub trait ValueCast {
    fn try_as_object(&self) -> HttpResult<&Map<String, Value>>;
    fn try_as_object_mut(&mut self) -> HttpResult<&mut Map<String, Value>>;

    fn try_as_array(&self) -> HttpResult<&Vec<Value>>;
    fn try_as_array_mut(&mut self) -> HttpResult<&mut Vec<Value>>;

    fn try_as_str(&self) -> HttpResult<&str>;
    fn try_as_number(&self) -> HttpResult<&Number>;
    fn try_as_i64(&self) -> HttpResult<i64>;
    fn try_as_u64(&self) -> HttpResult<u64>;
    fn try_as_f64(&self) -> HttpResult<f64>;
    fn try_as_bool(&self) -> HttpResult<bool>;
    fn try_as_null(&self) -> HttpResult<()>;
}

impl ValueCast for Value {
    fn try_as_object(&self) -> HttpResult<&Map<String, Value>> {
        match self.as_object() {
            Some(x) => Ok(x),
            None => Err(WebError::InvalidData(format!("Expected object. Got {}", get_kind_str(self))))
        }
    }

    fn try_as_object_mut(&mut self) -> HttpResult<&mut Map<String, Value>> {
        let kind = get_kind_str(self);
        match self.as_object_mut() {
            Some(x) => Ok(x),
            None => Err(WebError::InvalidData(format!("Expected object. Got {}", kind)))
        }
    }

    fn try_as_array(&self) -> HttpResult<&Vec<Value>> {
        match self.as_array() {
            Some(x) => Ok(x),
            None => Err(WebError::InvalidData(format!("Expected array. Got {}", get_kind_str(self))))
        }
    }

    fn try_as_array_mut(&mut self) -> HttpResult<&mut Vec<Value>> {
        let kind = get_kind_str(self);
        match self.as_array_mut() {
            Some(x) => Ok(x),
            None => Err(WebError::InvalidData(format!("Expected array. Got {}", kind)))
        }
    }

    fn try_as_str(&self) -> HttpResult<&str> {
        match self.as_str() {
            Some(x) => Ok(x),
            None => Err(WebError::InvalidData(format!("Expected string. Got {}", get_kind_str(self))))
        }
    }

    fn try_as_number(&self) -> HttpResult<&Number> {
        match self.as_number() {
            Some(x) => Ok(x),
            None => Err(WebError::InvalidData(format!("Expected number. Got {}", get_kind_str(self))))
        }
    }

    fn try_as_i64(&self) -> HttpResult<i64> {
        match self.as_i64() {
            Some(x) => Ok(x),
            None => Err(WebError::InvalidData(format!("Expected number. Got {}", get_kind_str(self))))
        }
    }

    fn try_as_u64(&self) -> HttpResult<u64> {
        match self.as_u64() {
            Some(x) => Ok(x),
            None => Err(WebError::InvalidData(format!("Expected number. Got {}", get_kind_str(self))))
        }
    }

    fn try_as_f64(&self) -> HttpResult<f64> {
        match self.as_f64() {
            Some(x) => Ok(x),
            None => Err(WebError::InvalidData(format!("Expected number. Got {}", get_kind_str(self))))
        }
    }

    fn try_as_bool(&self) -> HttpResult<bool> {
        match self.as_bool() {
            Some(x) => Ok(x),
            None => Err(WebError::InvalidData(format!("Expected boolean. Got {}", get_kind_str(self))))
        }
    }

    fn try_as_null(&self) -> HttpResult<()> {
        match self.as_null() {
            Some(x) => Ok(x),
            None => Err(WebError::InvalidData(format!("Expected null. Got {}", get_kind_str(self))))
        }
    }
}

pub trait HttpErrorMap {
    fn try_get(&self, key: &str) -> HttpResult<&Value>;
}

impl HttpErrorMap for Map<String, Value> {
    fn try_get(&self, key: &str) -> HttpResult<&Value> {
        match self.get(key) {
            Some(v) => Ok(v),
            None => Err(WebError::InvalidData(format!("Couldn't find key {}", key)))
        }
    }
}

pub fn parse_json(s: &str) -> HttpResult<Value> {
    match Value::from_str(s) {
        Ok(v) => Ok(v),
        Err(e) => Err(WebError::InvalidData(format!("Failed to parse json '{:?}", e)))
    }
}

pub fn parse_json_as_object(s: &str) -> HttpResult<Map<String, Value>> {
    let parsed: Value = parse_json(s)?;

    match parsed {
        Value::Object(o) => Ok(o),
        other => Err(WebError::InvalidData(format!("Expected object. Got {}", get_kind_str(&other))))
    }
}

pub fn parse_json_as_array(s: &str) -> HttpResult<Vec<Value>> {
    let parsed: Value = parse_json(s)?;
    
    match parsed {
        Value::Array(arr) => Ok(arr),
        other => Err(WebError::InvalidData(format!("Expected array. Got {}", get_kind_str(&other))))
    }
}

pub fn decode_utf8(bytes: Vec<u8>) -> HttpResult<String> {
    match String::from_utf8(bytes) {
        Ok(s) => Ok(s),
        Err(_) => return Err(WebError::InvalidData("Couldn't decode string. All text should be UTF-8".to_string()))
    }
}
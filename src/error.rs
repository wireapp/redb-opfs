use std::io::{self, ErrorKind};

use js_sys::{self, JsString, Object};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::DomException;

#[derive(Debug, derive_more::Display, derive_more::Error, derive_more::From)]
pub struct Error(pub(crate) io::Error);

impl From<Error> for JsValue {
    fn from(value: Error) -> Self {
        fn construct_error_stack(err: &dyn std::error::Error) -> js_sys::Error {
            let out = js_sys::Error::new(&err.to_string());
            if let Some(source) = err.source() {
                let cause = construct_error_stack(source);
                out.set_cause(&cause);
            }
            out
        }

        let stacked_error = construct_error_stack(&value);
        stacked_error.set_name(&format!("{}Error", value.0.kind()));
        stacked_error.into()
    }
}

impl From<JsValue> for Error {
    fn from(value: JsValue) -> Self {
        match value.dyn_ref::<DomException>() {
            Some(dom) => match dom.code() {
                DomException::NOT_FOUND_ERR => io::Error::from(ErrorKind::NotFound),
                DomException::NO_DATA_ALLOWED_ERR | DomException::NO_MODIFICATION_ALLOWED_ERR => {
                    io::Error::from(ErrorKind::PermissionDenied)
                }
                DomException::TYPE_MISMATCH_ERR => io::Error::other("type mismatch"),
                _ => {
                    let name = dom.name();
                    let message = dom.message();
                    io::Error::other(format!("{name}: {message}"))
                }
            },
            None => {
                let js_serialization = Object::from(value).to_string();
                let str = <JsString as ToString>::to_string(&js_serialization);
                io::Error::other(str)
            }
        }
        .into()
    }
}

impl Error {
    pub(crate) fn ad_hoc(err: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> Self {
        io::Error::other(err).into()
    }

    pub(crate) fn to_io(value: JsValue) -> io::Error {
        Self::from(value).0
    }
}

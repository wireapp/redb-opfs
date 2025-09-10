use wasm_bindgen::JsValue;
use wasm_bindgen_futures::js_sys;

#[derive(Debug, derive_more::Display, derive_more::Error)]
pub struct Error(std::io::Error);

impl<T> From<T> for Error
where
    T: Into<std::io::Error>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

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

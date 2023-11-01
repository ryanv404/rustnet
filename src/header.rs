use std::collections::BTreeMap;

pub mod names;
pub mod values;

pub use names::{header_names, HeaderName};
pub use values::HeaderValue;

pub type HeadersMap = BTreeMap<HeaderName, HeaderValue>;

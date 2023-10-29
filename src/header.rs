use std::collections::BTreeMap;

use crate::consts::{CACHE_CONTROL, CONTENT_LENGTH, CONTENT_TYPE};

pub mod names;
pub mod values;

pub use names::{HeaderName, header_names};
pub use values::HeaderValue;

pub fn default_headers() -> BTreeMap<HeaderName, HeaderValue> {
	BTreeMap::from([
		(CACHE_CONTROL, "no-cache".into()),
		(CONTENT_LENGTH, "0".into()),
		(CONTENT_TYPE, "text/plain; charset=UTF-8".into())
	])
}



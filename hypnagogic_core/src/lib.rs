#![warn(clippy::pedantic, clippy::cargo)]
// too many lines is a dumb metric
#![allow(clippy::too_many_lines)]
// as is fine, clippy is silly
#![allow(clippy::cast_lossless)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
// not actually going to be a published crate, useless to add
#![allow(clippy::cargo_common_metadata)]
// annoying
#![allow(clippy::module_name_repetitions)]
// allow this for now, but it's probably a bad idea
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
// sign conversion is fine
#![allow(clippy::cast_sign_loss)]
// error we can't do anything about because of dependancies
#![allow(clippy::multiple_crate_versions)]
// map makes less sense in some contexts
#![allow(clippy::bind_instead_of_map)]
// throws in cases where `` obfuscates what's going on (code links)
#![allow(clippy::doc_markdown)]

pub mod config;
pub mod generation;
pub mod operations;
pub mod util;

mod confirmation;
mod preferences;
mod schema;
mod validation;

pub(super) use validation::{validate, validate_rows};

#[cfg(test)]
mod tests;

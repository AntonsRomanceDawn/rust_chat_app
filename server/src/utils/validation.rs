use serde_json::json;
use tracing::{instrument, warn};

use crate::errors::{
    error::ApiErrorItem,
    error_codes::{self, PASSWORD_TOO_LONG, PASSWORD_TOO_WEAK},
};

#[instrument]
pub fn validate_username(username: &str) -> Vec<ApiErrorItem> {
    let mut errs = vec![];

    if username.is_empty() {
        warn!("Username is empty");
        errs.push(ApiErrorItem::new(error_codes::USERNAME_REQUIRED, None));
    }

    if username.len() < 3 {
        warn!("Username is too short: {}", username);
        errs.push(ApiErrorItem::new(
            error_codes::USERNAME_TOO_SHORT,
            json!({"min": 3}),
        ));
    }

    if username.len() > 32 {
        warn!("Username is too long: {}", username);
        errs.push(ApiErrorItem::new(
            error_codes::USERNAME_TOO_LONG,
            json!({"max": 32}),
        ));
    }

    errs
}

#[instrument(skip(password))]
pub fn validate_password(password: &str) -> Vec<ApiErrorItem> {
    let mut errs = vec![];

    if password.is_empty() {
        warn!("Password is empty");
        errs.push(ApiErrorItem::new(error_codes::PASSWORD_REQUIRED, None));
    }

    if password.len() < 6 {
        warn!("Password is too short");
        errs.push(ApiErrorItem::new(
            error_codes::PASSWORD_TOO_SHORT,
            json!({"min": 8}),
        ));
    }

    if password.len() > 32 {
        warn!("Password is too long");
        errs.push(ApiErrorItem::new(PASSWORD_TOO_LONG, json!({"max": 128})));
    }

    if !({
        let pw = password.chars();
        pw.clone().any(|c| c.is_ascii_lowercase())
            && pw.clone().any(|c| c.is_ascii_uppercase())
            && pw.clone().any(|c| c.is_ascii_digit())
            && pw.clone().any(|c| !c.is_ascii_alphanumeric())
    }) {
        warn!("Password is too weak");
        errs.push(ApiErrorItem::new(
            PASSWORD_TOO_WEAK,
            json!({
                "number_required": true,
                "special_character_required": true,
                "uppercase_required": true,
                "lowercase_required": true
            }),
        ));
    }

    errs
}

pub fn validate_confirm_password(password: &str, confirm: &str) -> Vec<ApiErrorItem> {
    let mut errs = vec![];

    if confirm.is_empty() {
        warn!("Confirm password is empty");
        errs.push(ApiErrorItem::new(
            error_codes::CONFIRM_PASSWORD_REQUIRED,
            None,
        ))
    }

    if password != confirm {
        warn!("Password and confirm password do not match");
        errs.push(ApiErrorItem::new(error_codes::PASSWORD_CONFLICT, None));
    }

    errs
}

use std::fmt::{self, Display, Formatter};

/// Error type for the crate. Wraps SDK failure messages, path conversion
/// errors, and invariant violations in a single stringly-typed variant:
/// there isn't much the caller can discriminate on that isn't already in
/// the message.
#[derive(Debug)]
pub struct Error(String);

impl Error {
    pub(crate) fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<cxx::Exception> for Error {
    fn from(e: cxx::Exception) -> Self {
        Self(e.what().to_string())
    }
}

/// Deprecated alias for [`Error`]. Kept so existing code continues to
/// compile; new code should use `Error` directly.
#[deprecated(since = "0.1.1", note = "renamed to `Error`")]
pub type BrawError = Error;

/// BMD HRESULT code, surfaced to callbacks. `is_ok()` iff `>= 0` per
/// standard COM convention.
///
/// Received as the `result` parameter on every
/// [`Callback`](crate::Callback) method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HResult(pub i32);

impl HResult {
    pub const S_OK: HResult = HResult(0);

    pub fn is_ok(self) -> bool {
        self.0 >= 0
    }
    pub fn is_err(self) -> bool {
        self.0 < 0
    }
}

impl Display for HResult {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "HRESULT=0x{:08x}", self.0 as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hresult_sign_classification() {
        assert!(HResult(0).is_ok());
        assert!(HResult(1).is_ok());
        assert!(HResult(i32::MAX).is_ok());
        assert!(HResult(-1).is_err());
        assert!(HResult(i32::MIN).is_err());
    }

    #[test]
    fn hresult_s_ok_is_zero() {
        assert_eq!(HResult::S_OK, HResult(0));
        assert!(HResult::S_OK.is_ok());
    }

    #[test]
    fn hresult_display_format() {
        assert_eq!(format!("{}", HResult(0)), "HRESULT=0x00000000");
        // E_POINTER from COM: 0x80004003 (negative as i32).
        assert_eq!(
            format!("{}", HResult(0x8000_4003u32 as i32)),
            "HRESULT=0x80004003"
        );
    }

    #[test]
    fn error_display_passes_through() {
        let e = Error::new("boom");
        assert_eq!(format!("{e}"), "boom");
    }
}

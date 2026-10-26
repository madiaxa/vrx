/// Decimal labels for the pre-1.0 release line. The running application uses
/// Cargo's package version directly through crate::VERSION.
pub fn label(minor: u64, patch: u64) -> String {
    format!("v0.{minor}.{patch}")
}

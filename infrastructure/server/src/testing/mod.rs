//! Test-support utilities. Compiled into test builds only — nothing here
//! ships to the runtime binaries.

#[cfg(test)]
pub mod http_stub;

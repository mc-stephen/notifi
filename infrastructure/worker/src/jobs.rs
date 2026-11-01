//! Every background job the worker runs — one file each, beside this
//! declaration. The sweep loop in `main.rs` calls `run()` on four of
//! them; `db_backup` runs on its own clock (plus the `--backup-now`
//! one-shot). The roster:
//! * [`broadcasts`] — due admin broadcasts (scheduled notifications)
//! * [`schedules`] — due flow schedules
//! * [`campaigns`] — campaign drains
//! * [`purge`] — fully erasing expired project deletions
//! * [`db_backup`] — the daily database dump-and-upload

pub mod broadcasts;
pub mod campaigns;
pub mod db_backup;
pub mod purge;
pub mod schedules;

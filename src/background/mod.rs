//! 非同期再生モード
//!
//! `sap -a audio.mp3` でバックグラウンドに再生用のプロセスを起動し、
//! `sap pause` / `sap resume` / `sap stop` で外から操作する。
pub mod control;
pub mod spawn;

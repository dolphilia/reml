use std::collections::BTreeMap;

use crate::runtime::{Signal, SignalError, SignalErrorKind, SignalInfo};
use crate::runtime::api::guard_capability;
use crate::stage::{StageId, StageRequirement};
use serde::{Deserialize, Serialize};

use super::process::ProcessId;

#[cfg(any(feature = "core_time", feature = "metrics"))]
use crate::time::{Duration, Timestamp};
#[cfg(not(any(feature = "core_time", feature = "metrics")))]
use std::time::{Duration, SystemTime as Timestamp};

const CAP_SIGNAL: &str = "core.signal";
const EFFECTS_SIGNAL: &[&str] = &["signal"];
const EFFECTS_SIGNAL_PROCESS: &[&str] = &["signal", "process"];
const EFFECTS_SIGNAL_BLOCKING: &[&str] = &["signal", "io.blocking"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignalPayload {
    UserData(i64),
    RealTime(i64),
    Custom(BTreeMap<String, String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalDetail {
    pub info: SignalInfo,
    pub timestamp: Option<Timestamp>,
    pub payload: Option<SignalPayload>,
    pub source_pid: Option<ProcessId>,
    pub raw_code: Option<i64>,
}

pub fn from_runtime_info(info: SignalInfo) -> SignalDetail {
    SignalDetail {
        info,
        timestamp: None,
        payload: None,
        source_pid: Some(info.sender),
        raw_code: None,
    }
}

pub fn send(_pid: ProcessId, _signal: Signal) -> Result<(), SignalError> {
    ensure_signal_capability(EFFECTS_SIGNAL_PROCESS)?;
    Err(SignalError::new(
        SignalErrorKind::RuntimeFailure,
        "signal send is not wired in this runtime",
    ))
}

pub fn wait(_signals: &[Signal], _timeout: Option<Duration>) -> Result<SignalDetail, SignalError> {
    ensure_signal_capability(EFFECTS_SIGNAL_BLOCKING)?;
    Err(SignalError::new(
        SignalErrorKind::RuntimeFailure,
        "signal wait is not wired in this runtime",
    ))
}

pub fn raise(_signal: Signal) -> Result<(), SignalError> {
    ensure_signal_capability(EFFECTS_SIGNAL)?;
    Err(SignalError::new(
        SignalErrorKind::RuntimeFailure,
        "signal raise is not wired in this runtime",
    ))
}

fn ensure_signal_capability(required_effects: &[&str]) -> Result<(), SignalError> {
    let requirement = StageRequirement::AtLeast(StageId::Experimental);
    guard_capability(CAP_SIGNAL, requirement, required_effects)
        .map(|_| ())
        .map_err(|err| SignalError::new(SignalErrorKind::Unsupported, err.detail().to_string()))
}

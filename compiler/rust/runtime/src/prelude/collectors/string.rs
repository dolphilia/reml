//! `StringCollector` の雛形。UTF-8 バッファを構築しつつ `effect {mem}` を記録する。

use std::fmt;

use super::super::iter::{EffectLabels, IterError};
use super::{
    CollectError, CollectErrorKind, CollectOutcome, Collector, CollectorAuditTrail,
    CollectorEffectMarkers, CollectorKind, CollectorStageProfile,
};

const PURE_EFFECTS: EffectLabels = EffectLabels {
    mem: false,
    mutating: false,
    debug: false,
    async_pending: false,
    audit: false,
    cell: false,
    rc: false,
    unicode: false,
    io: false,
    io_blocking: false,
    io_async: false,
    security: false,
    transfer: false,
    fs_sync: false,
    mem_bytes: 0,
    predicate_calls: 0,
    rc_ops: 0,
    time: false,
    time_calls: 0,
    io_blocking_calls: 0,
    io_async_calls: 0,
    fs_sync_calls: 0,
    security_events: 0,
};

pub struct StringCollector {
    buffer: Vec<u8>,
    validator: Utf8Validator,
    stage_profile: CollectorStageProfile,
    effects: EffectLabels,
    markers: CollectorEffectMarkers,
}

impl StringCollector {
    fn audit_trail(&self, source: &'static str) -> CollectorAuditTrail {
        CollectorAuditTrail::new(
            CollectorKind::String,
            self.stage_profile.snapshot(source),
            self.effects,
            self.markers,
        )
    }

    fn invalid_encoding(&self, offset: usize, byte: u8, detail: impl Into<String>) -> CollectError {
        CollectError::new(
            CollectErrorKind::InvalidEncoding,
            StringError::invalid_encoding(offset, byte, detail.into()).to_string(),
            self.audit_trail("StringCollector::push"),
        )
        .with_detail(format!("byte=0x{byte:02X}; offset={offset}"))
    }
}

impl Collector<u8, CollectOutcome<String>> for StringCollector {
    type Error = CollectError;

    fn new() -> Self
    where
        Self: Sized,
    {
        Self {
            buffer: Vec::new(),
            validator: Utf8Validator::default(),
            stage_profile: CollectorStageProfile::for_kind(CollectorKind::String),
            effects: PURE_EFFECTS,
            markers: CollectorEffectMarkers::default(),
        }
    }

    fn with_capacity(capacity: usize) -> Self
    where
        Self: Sized,
    {
        let mut collector = Self::new();
        collector.buffer.reserve(capacity);
        collector.effects.mem = true;
        collector.markers.record_mem_reservation(capacity);
        collector
    }

    fn push(&mut self, value: u8) -> Result<(), Self::Error> {
        let offset = self.buffer.len();
        self.validator
            .push_byte(value)
            .map_err(|error| self.invalid_encoding(offset, value, error.to_string()))?;

        self.buffer.push(value);
        self.effects.mem = true;
        self.effects.mutating = true;
        Ok(())
    }

    fn reserve(&mut self, additional: usize) -> Result<(), Self::Error> {
        if additional > 0 {
            self.markers.record_reserve(additional);
        }
        self.buffer.reserve(additional);
        self.effects.mem = true;
        self.effects.mutating = true;
        Ok(())
    }

    fn finish(mut self) -> CollectOutcome<String>
    where
        Self: Sized,
    {
        debug_assert!(
            self.validator.pending == 0,
            "StringCollector finished with incomplete UTF-8 sequence"
        );
        self.markers.record_finish();
        self.effects.mem = true;
        let audit = self.audit_trail("StringCollector::finish");
        let string = String::from_utf8(self.buffer).expect("UTF-8 validity was enforced");
        CollectOutcome::new(string, audit)
    }

    fn iter_error(self, error: IterError) -> Self::Error
    where
        Self: Sized,
    {
        let audit = self.audit_trail("StringCollector::iter_error");
        CollectError::new(
            CollectErrorKind::IteratorFailure,
            "iterator source reported an error during StringCollector::collect",
            audit,
        )
        .with_detail(format!("{error:?}"))
    }
}

#[derive(Debug, Clone, Copy)]
struct Utf8Validator {
    pending: usize,
    next_range: (u8, u8),
}

impl Default for Utf8Validator {
    fn default() -> Self {
        Self {
            pending: 0,
            next_range: (0x00, 0xFF),
        }
    }
}

impl Utf8Validator {
    fn push_byte(&mut self, byte: u8) -> Result<(), Utf8ValidationError> {
        if self.pending == 0 {
            self.validate_lead(byte)
        } else {
            self.validate_continuation(byte)
        }
    }

    fn validate_lead(&mut self, byte: u8) -> Result<(), Utf8ValidationError> {
        match byte {
            0x00..=0x7F => Ok(()),
            0xC2..=0xDF => {
                self.pending = 1;
                self.next_range = (0x80, 0xBF);
                Ok(())
            }
            0xE0 => {
                self.pending = 2;
                self.next_range = (0xA0, 0xBF);
                Ok(())
            }
            0xE1..=0xEC | 0xEE..=0xEF => {
                self.pending = 2;
                self.next_range = (0x80, 0xBF);
                Ok(())
            }
            0xED => {
                self.pending = 2;
                self.next_range = (0x80, 0x9F);
                Ok(())
            }
            0xF0 => {
                self.pending = 3;
                self.next_range = (0x90, 0xBF);
                Ok(())
            }
            0xF1..=0xF3 => {
                self.pending = 3;
                self.next_range = (0x80, 0xBF);
                Ok(())
            }
            0xF4 => {
                self.pending = 3;
                self.next_range = (0x80, 0x8F);
                Ok(())
            }
            _ => Err(Utf8ValidationError::InvalidLead),
        }
    }

    fn validate_continuation(&mut self, byte: u8) -> Result<(), Utf8ValidationError> {
        let (low, high) = self.next_range;
        if byte < low || byte > high {
            self.pending = 0;
            return Err(Utf8ValidationError::InvalidContinuation { low, high });
        }
        self.pending -= 1;
        self.next_range = (0x80, 0xBF);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct StringError {
    offset: usize,
    byte: u8,
    detail: String,
}

impl StringError {
    fn invalid_encoding(offset: usize, byte: u8, detail: impl Into<String>) -> Self {
        Self {
            offset,
            byte,
            detail: detail.into(),
        }
    }
}

impl fmt::Display for StringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid UTF-8 byte 0x{:02X} at offset {} ({})",
            self.byte, self.offset, self.detail
        )
    }
}

#[derive(Debug, Clone)]
enum Utf8ValidationError {
    InvalidLead,
    InvalidContinuation { low: u8, high: u8 },
}

impl fmt::Display for Utf8ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Utf8ValidationError::InvalidLead => write!(
                f,
                "expected ASCII or multi-byte lead (0x00..=0x7F or 0xC2..=0xF4)"
            ),
            Utf8ValidationError::InvalidContinuation { low, high } => write!(
                f,
                "expected continuation byte in 0x{:02X}..=0x{:02X}",
                low, high
            ),
        }
    }
}

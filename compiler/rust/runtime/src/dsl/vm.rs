//! Core.Dsl.Vm の最小実装。

use std::panic::{catch_unwind, AssertUnwindSafe};

use serde_json::{Map as JsonMap, Value};

use crate::dsl::{emit_audit, AuditPayload, AUDIT_DSL_VM_EXECUTE};
use crate::prelude::ensure::{DiagnosticSeverity, GuardDiagnostic, IntoDiagnostic};

/// バイトコード。
#[derive(Debug, Clone)]
pub struct Bytecode<Op> {
    pub ops: Vec<Op>,
}

/// VM 状態。
#[derive(Debug, Clone)]
pub struct VmState<Value> {
    pub stack: Vec<Value>,
    pub frames: Vec<CallFrame>,
}

/// コールフレーム。
#[derive(Debug, Clone, Copy)]
pub struct CallFrame {
    pub ip: usize,
}

/// VM エラー。
#[derive(Debug, Clone)]
pub struct VmError {
    pub kind: VmErrorKind,
    pub message: String,
}

impl VmError {
    pub fn new(kind: VmErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

/// VM エラー種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmErrorKind {
    Halted,
    InvalidOpcode,
    StackUnderflow,
    RuntimeFailure,
}

pub type VmResult<T> = Result<T, VmError>;

/// バイトコードビルダー。
#[derive(Debug, Clone)]
pub struct BytecodeBuilder<Op> {
    ops: Vec<Op>,
}

impl<Op> BytecodeBuilder<Op> {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    pub fn emit(mut self, op: Op) -> Self {
        self.ops.push(op);
        self
    }

    pub fn build(self) -> Bytecode<Op> {
        Bytecode { ops: self.ops }
    }
}

/// VM のトレースイベント。
#[derive(Debug, Clone)]
pub struct VmTraceEvent<Op> {
    pub ip: usize,
    pub op: Op,
}

/// VM 実行コア（Fetch-Decode-Execute）。
pub struct VmCore;

/// Core.Dsl.Vm の名前空間。
pub struct Vm;

impl VmCore {
    pub fn step<Op: Clone, Value>(
        code: &Bytecode<Op>,
        mut state: VmState<Value>,
        exec: &mut impl FnMut(VmState<Value>, Op) -> VmResult<VmState<Value>>,
        mut trace: Option<&mut dyn FnMut(VmTraceEvent<Op>)>,
    ) -> VmResult<(VmState<Value>, bool)> {
        let ip = state.frames.last().map(|frame| frame.ip).unwrap_or(0);
        let op = match code.ops.get(ip).cloned() {
            Some(op) => op,
            None => return Ok((state, false)),
        };
        if let Some(frame) = state.frames.last_mut() {
            frame.ip = ip;
        } else {
            state.frames.push(CallFrame { ip });
        }

        if let Some(trace_fn) = trace.as_mut() {
            (*trace_fn)(VmTraceEvent { ip, op: op.clone() });
        }

        let mut payload = AuditPayload::new(AUDIT_DSL_VM_EXECUTE);
        payload.insert("dsl.vm.ip", Value::from(ip as u64));
        emit_audit(payload);

        let state = catch_unwind(AssertUnwindSafe(|| exec(state, op))).unwrap_or_else(|_| {
            Err(VmError::new(
                VmErrorKind::RuntimeFailure,
                "vm execute panicked",
            ))
        })?;
        Ok((state, true))
    }

    pub fn run<Op: Clone, Value>(
        code: Bytecode<Op>,
        mut state: VmState<Value>,
        mut exec: impl FnMut(VmState<Value>, Op) -> VmResult<VmState<Value>>,
        mut trace: Option<&mut dyn FnMut(VmTraceEvent<Op>)>,
    ) -> VmResult<VmState<Value>> {
        loop {
            let (next_state, advanced) = VmCore::step(&code, state, &mut exec, trace.as_deref_mut())?;
            state = next_state;
            if !advanced {
                break;
            }
        }
        Ok(state)
    }
}

impl Vm {
    pub fn bytecode_builder<Op>() -> BytecodeBuilder<Op> {
        BytecodeBuilder::new()
    }

    pub fn run<Op: Clone, Value>(
        code: Bytecode<Op>,
        state: VmState<Value>,
        exec: impl FnMut(VmState<Value>, Op) -> VmResult<VmState<Value>>,
    ) -> VmResult<VmState<Value>> {
        VmCore::run(code, state, exec, None)
    }
}

impl IntoDiagnostic for VmError {
    fn into_diagnostic(self) -> GuardDiagnostic {
        let code = match self.kind {
            VmErrorKind::Halted => "dsl.vm.halted",
            VmErrorKind::InvalidOpcode => "dsl.vm.invalid_opcode",
            VmErrorKind::StackUnderflow => "dsl.vm.stack_underflow",
            VmErrorKind::RuntimeFailure => "dsl.vm.runtime_error",
        };
        GuardDiagnostic {
            code,
            domain: "dsl",
            severity: DiagnosticSeverity::Error,
            message: self.message,
            notes: Vec::new(),
            extensions: JsonMap::new(),
            audit_metadata: JsonMap::new(),
        }
    }
}

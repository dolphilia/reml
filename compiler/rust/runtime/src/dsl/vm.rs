//! Core.Dsl.Vm の最小実装。

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

/// Core.Dsl.Vm の名前空間。
pub struct Vm;

impl Vm {
    pub fn bytecode_builder<Op>() -> BytecodeBuilder<Op> {
        BytecodeBuilder::new()
    }

    pub fn run<Op: Clone, Value>(
        code: Bytecode<Op>,
        mut state: VmState<Value>,
        exec: impl Fn(VmState<Value>, Op) -> VmResult<VmState<Value>>,
    ) -> VmResult<VmState<Value>> {
        for (ip, op) in code.ops.iter().cloned().enumerate() {
            if let Some(frame) = state.frames.last_mut() {
                frame.ip = ip;
            }
            state = exec(state, op)?;
        }
        Ok(state)
    }
}

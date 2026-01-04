use std::sync::Arc;

use reml_runtime::ffi::dsl::{
    set_ffi_call_executor, FfiCallExecutor, FfiError, FfiErrorKind, FfiRawFn, FfiValue,
};

const EXECUTOR_ALREADY_SET_CODE: &str = "ffi.call.executor_already_set";
const FFI_CALL_FAILED_CODE: &str = "ffi.call.failed";

pub fn install_cli_ffi_executor() -> Result<(), FfiError> {
    let executor = Arc::new(CliFfiExecutor);
    match set_ffi_call_executor(executor) {
        Ok(()) => Ok(()),
        Err(err) => {
            if err.diagnostic_code() == Some(EXECUTOR_ALREADY_SET_CODE) {
                return Ok(());
            }
            Err(err)
        }
    }
}

struct CliFfiExecutor;

impl FfiCallExecutor for CliFfiExecutor {
    fn call(&self, raw: &FfiRawFn, args: &[FfiValue]) -> Result<FfiValue, FfiError> {
        let library = raw.library_label();
        let symbol = raw.symbol();
        if matches!(library, "m" | "libm") && symbol == "cos" {
            return call_libm_cos(args);
        }
        Err(call_failed_error(format!(
            "FFI 呼び出しが未実装です: {library}::{symbol}"
        )))
    }
}

fn call_libm_cos(args: &[FfiValue]) -> Result<FfiValue, FfiError> {
    if args.len() != 1 {
        return Err(call_failed_error("cos は引数 1 つを要求します"));
    }
    match &args[0] {
        FfiValue::F64(value) => Ok(FfiValue::F64(value.cos())),
        FfiValue::F32(value) => Ok(FfiValue::F32(value.cos())),
        _ => Err(call_failed_error("cos の引数は F32/F64 が必要です")),
    }
}

fn call_failed_error(message: impl Into<String>) -> FfiError {
    FfiError::new(FfiErrorKind::CallFailed, message).with_code(FFI_CALL_FAILED_CODE)
}

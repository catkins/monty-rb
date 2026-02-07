use magnus::value::ReprValue;
use magnus::{method, Error, Module, RArray, Ruby, Value};
use monty::{
    CollectStringPrint, ExternalResult, FutureSnapshot, MontyObject, NoLimitTracker, RunProgress,
    Snapshot,
};
use std::cell::RefCell;

use crate::errors::{consumed_error, map_monty_exception, monty_error};
use crate::monty_object::{monty_to_ruby, ruby_to_monty};

/// Ruby wrapper for RunProgress - represents the state of iterative execution.
///
/// When execution hits an external function call, it pauses and returns a
/// FunctionCall progress. The caller resolves the function and resumes execution.
#[magnus::wrap(class = "Monty::FunctionCall", free_immediately, size)]
pub struct FunctionCall {
    function_name: String,
    args: Vec<MontyObject>,
    kwargs: Vec<(MontyObject, MontyObject)>,
    call_id: u32,
    output: String,
    state: RefCell<Option<Snapshot<NoLimitTracker>>>,
}

impl FunctionCall {
    fn function_name(&self) -> String {
        self.function_name.clone()
    }

    fn call_id(&self) -> u32 {
        self.call_id
    }

    fn output(&self) -> String {
        self.output.clone()
    }

    fn args(&self) -> Result<Value, Error> {
        let ruby = Ruby::get().expect("Ruby runtime not available");
        let arr = ruby.ary_new_capa(self.args.len());
        for arg in &self.args {
            arr.push(monty_to_ruby(arg.clone())?)?;
        }
        Ok(arr.as_value())
    }

    fn kwargs(&self) -> Result<Value, Error> {
        let ruby = Ruby::get().expect("Ruby runtime not available");
        let hash = ruby.hash_new();
        for (k, v) in &self.kwargs {
            hash.aset(monty_to_ruby(k.clone())?, monty_to_ruby(v.clone())?)?;
        }
        Ok(hash.as_value())
    }

    /// Resume execution by providing the return value of the external function.
    /// Consumes this FunctionCall — it cannot be used again.
    fn resume(&self, result: Value) -> Result<Progress, Error> {
        let snapshot = self
            .state
            .borrow_mut()
            .take()
            .ok_or_else(consumed_error)?;

        let monty_result = ruby_to_monty(result)?;
        let mut print = CollectStringPrint::new();

        let progress = snapshot
            .run(monty_result, &mut print)
            .map_err(map_monty_exception)?;

        Progress::from_run_progress(progress, print.into_output())
    }

    /// Resume execution by raising an exception in the Python code.
    /// Consumes this FunctionCall — it cannot be used again.
    fn resume_with_error(&self, message: String) -> Result<Progress, Error> {
        let snapshot = self
            .state
            .borrow_mut()
            .take()
            .ok_or_else(consumed_error)?;

        let exc = monty::MontyException::new(monty::ExcType::RuntimeError, Some(message));
        let mut print = CollectStringPrint::new();

        let progress = snapshot
            .run(ExternalResult::Error(exc), &mut print)
            .map_err(map_monty_exception)?;

        Progress::from_run_progress(progress, print.into_output())
    }
}

/// Represents pending async futures that need resolution
#[magnus::wrap(class = "Monty::PendingFutures", free_immediately, size)]
pub struct PendingFutures {
    pending_call_ids: Vec<u32>,
    output: String,
    state: RefCell<Option<FutureSnapshot<NoLimitTracker>>>,
}

impl PendingFutures {
    fn pending_call_ids(&self) -> Result<Value, Error> {
        let ruby = Ruby::get().expect("Ruby runtime not available");
        let arr = ruby.ary_new_capa(self.pending_call_ids.len());
        for id in &self.pending_call_ids {
            arr.push(ruby.integer_from_u64(*id as u64))?;
        }
        Ok(arr.as_value())
    }

    fn output(&self) -> String {
        self.output.clone()
    }

    /// Resume execution by providing results for pending futures.
    /// `results` is an Array of [call_id, value] pairs.
    /// Consumes this PendingFutures — it cannot be used again.
    fn resume(&self, results: RArray) -> Result<Progress, Error> {
        let snapshot = self
            .state
            .borrow_mut()
            .take()
            .ok_or_else(consumed_error)?;

        let mut resolved = Vec::with_capacity(results.len());
        for i in 0..results.len() {
            let pair: RArray = results.entry(i as isize)?;
            if pair.len() != 2 {
                return Err(monty_error(
                    "each result must be a [call_id, value] pair".to_string(),
                ));
            }
            let call_id: u32 = pair.entry(0)?;
            let value: Value = pair.entry(1)?;
            let monty_value = ruby_to_monty(value)?;
            resolved.push((call_id, ExternalResult::Return(monty_value)));
        }

        let mut print = CollectStringPrint::new();

        let progress = snapshot
            .resume(resolved, &mut print)
            .map_err(map_monty_exception)?;

        Progress::from_run_progress(progress, print.into_output())
    }
}

/// Represents a completed execution with its result value
#[magnus::wrap(class = "Monty::Complete", free_immediately, size)]
pub struct Complete {
    result: RefCell<Option<MontyObject>>,
    output: String,
}

impl Complete {
    fn value(&self) -> Result<Value, Error> {
        let obj = self
            .result
            .borrow_mut()
            .take()
            .ok_or_else(consumed_error)?;
        monty_to_ruby(obj)
    }

    fn output(&self) -> String {
        self.output.clone()
    }
}

/// Unified progress result returned from start/resume operations
pub enum Progress {
    FunctionCall(FunctionCall),
    PendingFutures(PendingFutures),
    Complete(Complete),
}

impl Progress {
    pub fn from_run_progress(
        progress: RunProgress<NoLimitTracker>,
        output: String,
    ) -> Result<Self, Error> {
        match progress {
            RunProgress::FunctionCall {
                function_name,
                args,
                kwargs,
                call_id,
                state,
            } => Ok(Progress::FunctionCall(FunctionCall {
                function_name,
                args,
                kwargs,
                call_id,
                output,
                state: RefCell::new(Some(state)),
            })),
            RunProgress::OsCall {
                function,
                args,
                kwargs,
                call_id,
                state,
            } => {
                // Map OsCall as a FunctionCall with the function name
                Ok(Progress::FunctionCall(FunctionCall {
                    function_name: format!("os:{function:?}"),
                    args,
                    kwargs,
                    call_id,
                    output,
                    state: RefCell::new(Some(state)),
                }))
            }
            RunProgress::ResolveFutures(snapshot) => {
                let pending_ids = snapshot.pending_call_ids().to_vec();
                Ok(Progress::PendingFutures(PendingFutures {
                    pending_call_ids: pending_ids,
                    output,
                    state: RefCell::new(Some(snapshot)),
                }))
            }
            RunProgress::Complete(obj) => Ok(Progress::Complete(Complete {
                result: RefCell::new(Some(obj)),
                output,
            })),
        }
    }
}

impl magnus::IntoValue for Progress {
    fn into_value_with(self, handle: &Ruby) -> Value {
        match self {
            Progress::FunctionCall(fc) => handle.into_value(fc),
            Progress::PendingFutures(pf) => handle.into_value(pf),
            Progress::Complete(c) => handle.into_value(c),
        }
    }
}

pub fn define_progress_classes(ruby: &Ruby, module: &magnus::RModule) -> Result<(), Error> {
    // FunctionCall class
    let fc_class = module.define_class("FunctionCall", ruby.class_object())?;
    fc_class.define_method("function_name", method!(FunctionCall::function_name, 0))?;
    fc_class.define_method("call_id", method!(FunctionCall::call_id, 0))?;
    fc_class.define_method("args", method!(FunctionCall::args, 0))?;
    fc_class.define_method("kwargs", method!(FunctionCall::kwargs, 0))?;
    fc_class.define_method("output", method!(FunctionCall::output, 0))?;
    fc_class.define_method("resume", method!(FunctionCall::resume, 1))?;
    fc_class.define_method(
        "resume_with_error",
        method!(FunctionCall::resume_with_error, 1),
    )?;

    // PendingFutures class
    let pf_class = module.define_class("PendingFutures", ruby.class_object())?;
    pf_class.define_method(
        "pending_call_ids",
        method!(PendingFutures::pending_call_ids, 0),
    )?;
    pf_class.define_method("output", method!(PendingFutures::output, 0))?;
    pf_class.define_method("resume", method!(PendingFutures::resume, 1))?;

    // Complete class
    let complete_class = module.define_class("Complete", ruby.class_object())?;
    complete_class.define_method("value", method!(Complete::value, 0))?;
    complete_class.define_method("output", method!(Complete::output, 0))?;

    Ok(())
}

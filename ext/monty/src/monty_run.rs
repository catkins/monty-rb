use magnus::value::ReprValue;
use magnus::{function, method, Error, Module, Object, RArray, RHash, Ruby, Value};
use monty_lang::{CollectStringPrint, LimitedTracker, MontyRun, NoLimitTracker, StdPrint};
use std::cell::RefCell;

use crate::errors::{consumed_error, map_monty_exception};
use crate::monty_object::{monty_to_ruby, ruby_array_to_monty_vec};
use crate::resource_limits::parse_limits_hash;
use crate::run_progress::Progress;

/// Ruby wrapper for monty::MontyRun
///
/// Uses RefCell<Option<>> to support the consuming `start` method.
/// After `start` is called, the inner MontyRun is taken and the
/// Run object can no longer be used for `run` or `start`.
#[magnus::wrap(class = "Monty::Run", free_immediately, size)]
pub struct Run {
    inner: RefCell<Option<MontyRun>>,
}

impl Run {
    /// Parse Python code and create a reusable executor.
    ///
    /// Arguments:
    ///   code       - Python source code string
    ///   script_name - filename for error messages (default: "script.py")
    ///   inputs     - Array of input variable names (default: [])
    ///   external_functions - Array of external function names (default: [])
    fn new(
        code: String,
        script_name: Option<String>,
        inputs: Option<RArray>,
        external_functions: Option<RArray>,
    ) -> Result<Self, Error> {
        let script_name = script_name.unwrap_or_else(|| "script.py".to_string());

        let input_names: Vec<String> = match inputs {
            Some(arr) => {
                let mut names = Vec::with_capacity(arr.len());
                for i in 0..arr.len() {
                    let val: String = arr.entry(i as isize)?;
                    names.push(val);
                }
                names
            }
            None => Vec::new(),
        };

        let ext_fns: Vec<String> = match external_functions {
            Some(arr) => {
                let mut names = Vec::with_capacity(arr.len());
                for i in 0..arr.len() {
                    let val: String = arr.entry(i as isize)?;
                    names.push(val);
                }
                names
            }
            None => Vec::new(),
        };

        let monty_run = MontyRun::new(code, &script_name, input_names, ext_fns)
            .map_err(map_monty_exception)?;

        Ok(Self {
            inner: RefCell::new(Some(monty_run)),
        })
    }

    /// Get the source code
    fn code(&self) -> Result<String, Error> {
        let inner = self.inner.borrow();
        let run = inner.as_ref().ok_or_else(consumed_error)?;
        Ok(run.code().to_string())
    }

    /// Execute the Python code with inputs, no resource limits.
    /// Prints to stdout directly.
    fn run(&self, inputs: RArray) -> Result<Value, Error> {
        let inner = self.inner.borrow();
        let run = inner.as_ref().ok_or_else(consumed_error)?;

        let monty_inputs = ruby_array_to_monty_vec(inputs)?;
        let result = run
            .run(monty_inputs, NoLimitTracker, &mut StdPrint)
            .map_err(map_monty_exception)?;

        monty_to_ruby(result)
    }

    /// Execute the Python code with inputs and resource limits.
    /// Prints to stdout directly.
    fn run_with_limits(&self, inputs: RArray, limits: RHash) -> Result<Value, Error> {
        let inner = self.inner.borrow();
        let run = inner.as_ref().ok_or_else(consumed_error)?;

        let monty_inputs = ruby_array_to_monty_vec(inputs)?;
        let resource_limits = parse_limits_hash(&limits)?;
        let tracker = LimitedTracker::new(resource_limits);

        let result = run
            .run(monty_inputs, tracker, &mut StdPrint)
            .map_err(map_monty_exception)?;

        monty_to_ruby(result)
    }

    /// Execute the Python code and capture stdout output.
    /// Returns a Hash with :result and :output keys.
    fn run_capturing(&self, inputs: RArray) -> Result<Value, Error> {
        let ruby = Ruby::get().expect("Ruby runtime not available");
        let inner = self.inner.borrow();
        let run = inner.as_ref().ok_or_else(consumed_error)?;

        let monty_inputs = ruby_array_to_monty_vec(inputs)?;
        let mut print = CollectStringPrint::new();

        let result = run
            .run(monty_inputs, NoLimitTracker, &mut print)
            .map_err(map_monty_exception)?;

        let hash = ruby.hash_new();
        hash.aset(ruby.to_symbol("result"), monty_to_ruby(result)?)?;
        hash.aset(
            ruby.to_symbol("output"),
            ruby.str_new(print.output()),
        )?;
        Ok(hash.as_value())
    }

    /// Execute the Python code with resource limits and capture stdout.
    /// Returns a Hash with :result and :output keys.
    fn run_capturing_with_limits(&self, inputs: RArray, limits: RHash) -> Result<Value, Error> {
        let ruby = Ruby::get().expect("Ruby runtime not available");
        let inner = self.inner.borrow();
        let run = inner.as_ref().ok_or_else(consumed_error)?;

        let monty_inputs = ruby_array_to_monty_vec(inputs)?;
        let resource_limits = parse_limits_hash(&limits)?;
        let tracker = LimitedTracker::new(resource_limits);
        let mut print = CollectStringPrint::new();

        let result = run
            .run(monty_inputs, tracker, &mut print)
            .map_err(map_monty_exception)?;

        let hash = ruby.hash_new();
        hash.aset(ruby.to_symbol("result"), monty_to_ruby(result)?)?;
        hash.aset(
            ruby.to_symbol("output"),
            ruby.str_new(print.output()),
        )?;
        Ok(hash.as_value())
    }

    /// Start iterative execution (for external function calls).
    /// Consumes the Run — it cannot be used again after this.
    fn start(&self, inputs: RArray) -> Result<Progress, Error> {
        let monty_run = self
            .inner
            .borrow_mut()
            .take()
            .ok_or_else(consumed_error)?;

        let monty_inputs = ruby_array_to_monty_vec(inputs)?;
        let mut print = CollectStringPrint::new();

        let progress = monty_run
            .start(monty_inputs, NoLimitTracker, &mut print)
            .map_err(map_monty_exception)?;

        Progress::from_run_progress(progress, print.into_output())
    }

    /// Serialize the Run to bytes
    fn dump(&self) -> Result<Vec<u8>, Error> {
        let inner = self.inner.borrow();
        let run = inner.as_ref().ok_or_else(consumed_error)?;
        run.dump().map_err(|e| {
            let ruby = Ruby::get().expect("Ruby runtime not available");
            Error::new(
                ruby.exception_runtime_error(),
                format!("serialization error: {e}"),
            )
        })
    }

    /// Deserialize a Run from bytes
    fn load(bytes: Vec<u8>) -> Result<Self, Error> {
        let monty_run = MontyRun::load(&bytes).map_err(|e| {
            let ruby = Ruby::get().expect("Ruby runtime not available");
            Error::new(
                ruby.exception_runtime_error(),
                format!("deserialization error: {e}"),
            )
        })?;

        Ok(Self {
            inner: RefCell::new(Some(monty_run)),
        })
    }
}

pub fn define_run_class(ruby: &Ruby, module: &magnus::RModule) -> Result<(), Error> {
    let class = module.define_class("Run", ruby.class_object())?;

    class.define_singleton_method("_new", function!(Run::new, 4))?;
    class.define_singleton_method("_load", function!(Run::load, 1))?;

    class.define_method("code", method!(Run::code, 0))?;
    class.define_method("_run", method!(Run::run, 1))?;
    class.define_method("_run_with_limits", method!(Run::run_with_limits, 2))?;
    class.define_method("_run_capturing", method!(Run::run_capturing, 1))?;
    class.define_method(
        "_run_capturing_with_limits",
        method!(Run::run_capturing_with_limits, 2),
    )?;
    class.define_method("_start", method!(Run::start, 1))?;
    class.define_method("_dump", method!(Run::dump, 0))?;

    Ok(())
}

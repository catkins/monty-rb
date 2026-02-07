use magnus::{Error, ExceptionClass, Module, Ruby};
use std::cell::RefCell;

thread_local! {
    static MONTY_ERROR: RefCell<Option<ExceptionClass>> = const { RefCell::new(None) };
    static SYNTAX_ERROR: RefCell<Option<ExceptionClass>> = const { RefCell::new(None) };
    static RESOURCE_ERROR: RefCell<Option<ExceptionClass>> = const { RefCell::new(None) };
    static CONSUMED_ERROR: RefCell<Option<ExceptionClass>> = const { RefCell::new(None) };
}

pub fn define_exceptions(ruby: &Ruby, module: &magnus::RModule) -> Result<(), Error> {
    let standard_error = ruby.exception_standard_error();

    let monty_error = module.define_error("Error", standard_error)?;
    MONTY_ERROR.with(|cell| {
        *cell.borrow_mut() = Some(monty_error);
    });

    let syntax_error = module.define_error("SyntaxError", monty_error)?;
    SYNTAX_ERROR.with(|cell| {
        *cell.borrow_mut() = Some(syntax_error);
    });

    let resource_error = module.define_error("ResourceError", monty_error)?;
    RESOURCE_ERROR.with(|cell| {
        *cell.borrow_mut() = Some(resource_error);
    });

    let consumed_error = module.define_error("ConsumedError", monty_error)?;
    CONSUMED_ERROR.with(|cell| {
        *cell.borrow_mut() = Some(consumed_error);
    });

    Ok(())
}

pub fn monty_error(message: String) -> Error {
    MONTY_ERROR.with(|cell| {
        let class = cell.borrow();
        match class.as_ref() {
            Some(cls) => Error::new(*cls, message),
            None => {
                let ruby = Ruby::get().expect("Ruby runtime not available");
                Error::new(ruby.exception_runtime_error(), message)
            },
        }
    })
}

pub fn syntax_error(message: String) -> Error {
    SYNTAX_ERROR.with(|cell| {
        let class = cell.borrow();
        match class.as_ref() {
            Some(cls) => Error::new(*cls, message),
            None => {
                let ruby = Ruby::get().expect("Ruby runtime not available");
                Error::new(ruby.exception_runtime_error(), message)
            },
        }
    })
}

pub fn resource_error(message: String) -> Error {
    RESOURCE_ERROR.with(|cell| {
        let class = cell.borrow();
        match class.as_ref() {
            Some(cls) => Error::new(*cls, message),
            None => {
                let ruby = Ruby::get().expect("Ruby runtime not available");
                Error::new(ruby.exception_runtime_error(), message)
            },
        }
    })
}

pub fn consumed_error() -> Error {
    CONSUMED_ERROR.with(|cell| {
        let class = cell.borrow();
        match class.as_ref() {
            Some(cls) => Error::new(
                *cls,
                "this object has been consumed and can no longer be used",
            ),
            None => {
                let ruby = Ruby::get().expect("Ruby runtime not available");
                Error::new(
                    ruby.exception_runtime_error(),
                    "this object has been consumed and can no longer be used",
                )
            }
        }
    })
}

pub fn map_monty_exception(exc: monty::MontyException) -> Error {
    let summary = exc.summary();

    // Check if it's a syntax error
    if exc.exc_type() == monty::ExcType::SyntaxError {
        return syntax_error(summary);
    }

    monty_error(summary)
}

pub fn map_resource_error(err: monty::ResourceError) -> Error {
    let message = match err {
        monty::ResourceError::Allocation { limit, count } => {
            format!("allocation limit exceeded: {count} allocations (limit: {limit})")
        }
        monty::ResourceError::Time { limit, elapsed } => {
            format!(
                "time limit exceeded: {:.2}s elapsed (limit: {:.2}s)",
                elapsed.as_secs_f64(),
                limit.as_secs_f64()
            )
        }
        monty::ResourceError::Memory { limit, used } => {
            format!("memory limit exceeded: {used} bytes used (limit: {limit})")
        }
        monty::ResourceError::Recursion { limit, depth } => {
            format!("recursion limit exceeded: depth {depth} (limit: {limit})")
        }
        monty::ResourceError::Exception(exc) => {
            return map_monty_exception(exc);
        }
    };

    resource_error(message)
}

use magnus::{Error, Ruby};

#[allow(dead_code)]
mod errors;
mod monty_object;
mod monty_run;
mod resource_limits;
mod run_progress;

#[magnus::init]
fn init(ruby: &Ruby) -> Result<(), Error> {
    let module = ruby.define_module("Monty")?;

    errors::define_exceptions(ruby, &module)?;
    resource_limits::define_resource_limits_class(ruby, &module)?;
    monty_run::define_run_class(ruby, &module)?;
    run_progress::define_progress_classes(ruby, &module)?;

    Ok(())
}

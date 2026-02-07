use magnus::value::ReprValue;
use magnus::{function, Error, Module, Object, RHash, Ruby, TryConvert, Value};
use std::time::Duration;

/// Ruby wrapper for monty::ResourceLimits
#[magnus::wrap(class = "Monty::ResourceLimits", free_immediately, size)]
pub struct ResourceLimits {
    #[allow(dead_code)]
    pub inner: monty::ResourceLimits,
}

impl ResourceLimits {
    fn new(options: Option<RHash>) -> Result<Self, Error> {
        let mut limits = monty::ResourceLimits::new();

        if let Some(opts) = options {
            if let Some(val) = get_optional_usize(&opts, "max_allocations")? {
                limits = limits.max_allocations(val);
            }
            if let Some(val) = get_optional_f64(&opts, "max_duration")? {
                limits = limits.max_duration(Duration::from_secs_f64(val));
            }
            if let Some(val) = get_optional_usize(&opts, "max_memory")? {
                limits = limits.max_memory(val);
            }
            if let Some(val) = get_optional_usize(&opts, "gc_interval")? {
                limits = limits.gc_interval(val);
            }
            if let Some(val) = get_optional_usize(&opts, "max_recursion_depth")? {
                limits = limits.max_recursion_depth(Some(val));
            }
        }

        Ok(Self { inner: limits })
    }
}

pub fn define_resource_limits_class(ruby: &Ruby, module: &magnus::RModule) -> Result<(), Error> {
    let class = module.define_class("ResourceLimits", ruby.class_object())?;

    class.define_singleton_method("new", function!(ResourceLimits::new, 1))?;

    Ok(())
}

pub fn parse_limits_hash(opts: &RHash) -> Result<monty::ResourceLimits, Error> {
    let mut limits = monty::ResourceLimits::new();

    if let Some(val) = get_optional_usize(opts, "max_allocations")? {
        limits = limits.max_allocations(val);
    }
    if let Some(val) = get_optional_f64(opts, "max_duration")? {
        limits = limits.max_duration(Duration::from_secs_f64(val));
    }
    if let Some(val) = get_optional_usize(opts, "max_memory")? {
        limits = limits.max_memory(val);
    }
    if let Some(val) = get_optional_usize(opts, "gc_interval")? {
        limits = limits.gc_interval(val);
    }
    if let Some(val) = get_optional_usize(opts, "max_recursion_depth")? {
        limits = limits.max_recursion_depth(Some(val));
    }

    Ok(limits)
}

fn get_optional_usize(hash: &RHash, key: &str) -> Result<Option<usize>, Error> {
    let ruby = Ruby::get().expect("Ruby runtime not available");
    let sym = ruby.to_symbol(key);
    let val: Value = hash.aref(sym)?;
    if val.is_nil() {
        Ok(None)
    } else {
        Ok(Some(usize::try_convert(val)?))
    }
}

fn get_optional_f64(hash: &RHash, key: &str) -> Result<Option<f64>, Error> {
    let ruby = Ruby::get().expect("Ruby runtime not available");
    let sym = ruby.to_symbol(key);
    let val: Value = hash.aref(sym)?;
    if val.is_nil() {
        Ok(None)
    } else {
        Ok(Some(f64::try_convert(val)?))
    }
}

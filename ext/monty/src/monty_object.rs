use magnus::value::ReprValue;
use magnus::{Error, RArray, RHash, Ruby, TryConvert, Value};
use monty_lang::MontyObject;

/// Convert a Ruby value to a MontyObject
pub fn ruby_to_monty(val: Value) -> Result<MontyObject, Error> {
    let ruby = Ruby::get().expect("Ruby runtime not available");

    if val.is_nil() {
        return Ok(MontyObject::None);
    }

    // Check for booleans by class name
    if let Some(b) = detect_bool(val) {
        return Ok(MontyObject::Bool(b));
    }

    // Integer
    if val.is_kind_of(ruby.class_integer()) {
        // Try i64 first, fall back to BigInt via string
        if let Ok(i) = i64::try_convert(val) {
            return Ok(MontyObject::Int(i));
        }
        // Large integer: convert via string representation
        let s: String = val.funcall("to_s", ())?;
        let big = s
            .parse::<num_bigint::BigInt>()
            .map_err(|e| Error::new(ruby.exception_arg_error(), format!("invalid integer: {e}")))?;
        return Ok(MontyObject::BigInt(big));
    }

    // Float
    if val.is_kind_of(ruby.class_float()) {
        let f: f64 = f64::try_convert(val)?;
        return Ok(MontyObject::Float(f));
    }

    // String
    if val.is_kind_of(ruby.class_string()) {
        let s: String = String::try_convert(val)?;
        return Ok(MontyObject::String(s));
    }

    // Symbol -> String
    if val.is_kind_of(ruby.class_symbol()) {
        let s: String = val.funcall("to_s", ())?;
        return Ok(MontyObject::String(s));
    }

    // Array -> List
    if val.is_kind_of(ruby.class_array()) {
        let arr: RArray = RArray::try_convert(val)?;
        let mut items = Vec::with_capacity(arr.len());
        for i in 0..arr.len() {
            let item: Value = arr.entry(i as isize)?;
            items.push(ruby_to_monty(item)?);
        }
        return Ok(MontyObject::List(items));
    }

    // Hash -> Dict
    if val.is_kind_of(ruby.class_hash()) {
        let hash: RHash = RHash::try_convert(val)?;
        let pairs = hash_to_pairs(hash)?;
        return Ok(MontyObject::dict(pairs));
    }

    Err(Error::new(
        ruby.exception_type_error(),
        format!(
            "cannot convert {} to a Python object",
            val.class().inspect()
        ),
    ))
}

/// Convert a MontyObject to a Ruby value
pub fn monty_to_ruby(obj: MontyObject) -> Result<Value, Error> {
    let ruby = Ruby::get().expect("Ruby runtime not available");

    match obj {
        MontyObject::None => Ok(ruby.qnil().as_value()),
        MontyObject::Bool(b) => {
            if b {
                Ok(ruby.qtrue().as_value())
            } else {
                Ok(ruby.qfalse().as_value())
            }
        }
        MontyObject::Int(i) => Ok(ruby.integer_from_i64(i).as_value()),
        MontyObject::BigInt(bi) => {
            let s = bi.to_string();
            let ruby_str = ruby.str_new(&s);
            ruby_str.funcall("to_i", ())
        }
        MontyObject::Float(f) => Ok(ruby.float_from_f64(f).as_value()),
        MontyObject::String(s) => Ok(ruby.str_new(&s).as_value()),
        MontyObject::Bytes(b) => {
            let s = ruby.str_from_slice(&b);
            s.funcall::<_, _, Value>("force_encoding", ("ASCII-8BIT",))?;
            Ok(s.as_value())
        }
        MontyObject::List(items) => {
            let arr = ruby.ary_new_capa(items.len());
            for item in items {
                let val = monty_to_ruby(item)?;
                arr.push(val)?;
            }
            Ok(arr.as_value())
        }
        MontyObject::Tuple(items) => {
            let arr = ruby.ary_new_capa(items.len());
            for item in items {
                let val = monty_to_ruby(item)?;
                arr.push(val)?;
            }
            arr.funcall::<_, _, Value>("freeze", ())?;
            Ok(arr.as_value())
        }
        MontyObject::NamedTuple {
            field_names,
            values,
            ..
        } => {
            let hash = ruby.hash_new();
            for (name, value) in field_names.into_iter().zip(values.into_iter()) {
                let key = ruby.str_new(&name);
                let val = monty_to_ruby(value)?;
                hash.aset(key, val)?;
            }
            Ok(hash.as_value())
        }
        MontyObject::Dict(pairs) => {
            let hash = ruby.hash_new();
            for (k, v) in pairs.into_iter() {
                let key = monty_to_ruby(k)?;
                let val = monty_to_ruby(v)?;
                hash.aset(key, val)?;
            }
            Ok(hash.as_value())
        }
        MontyObject::Set(items) | MontyObject::FrozenSet(items) => {
            let arr = ruby.ary_new_capa(items.len());
            for item in items {
                let val = monty_to_ruby(item)?;
                arr.push(val)?;
            }
            Ok(arr.as_value())
        }
        MontyObject::Dataclass { attrs, .. } => {
            let hash = ruby.hash_new();
            for (k, v) in attrs.into_iter() {
                let key = monty_to_ruby(k)?;
                let val = monty_to_ruby(v)?;
                hash.aset(key, val)?;
            }
            Ok(hash.as_value())
        }
        MontyObject::Ellipsis => {
            let sym = ruby.to_symbol("ellipsis");
            Ok(sym.as_value())
        }
        MontyObject::Type(t) => {
            let repr = format!("{t:?}");
            Ok(ruby.str_new(&repr).as_value())
        }
        MontyObject::BuiltinFunction(f) => {
            let repr = format!("{f:?}");
            Ok(ruby.str_new(&repr).as_value())
        }
        MontyObject::Path(s) => Ok(ruby.str_new(&s).as_value()),
        MontyObject::Repr(s) => Ok(ruby.str_new(&s).as_value()),
        MontyObject::Cycle(_, s) => Ok(ruby.str_new(&s).as_value()),
        MontyObject::Exception { exc_type, arg } => {
            let msg = arg.unwrap_or_else(|| format!("{exc_type:?}"));
            Err(crate::errors::monty_error(msg))
        }
    }
}

/// Convert a Ruby Array of values to Vec<MontyObject>
pub fn ruby_array_to_monty_vec(arr: RArray) -> Result<Vec<MontyObject>, Error> {
    let mut result = Vec::with_capacity(arr.len());
    for i in 0..arr.len() {
        let item: Value = arr.entry(i as isize)?;
        result.push(ruby_to_monty(item)?);
    }
    Ok(result)
}

/// Detect Ruby true/false by querying the class name
fn detect_bool(val: Value) -> Option<bool> {
    let class_val: Value = val.funcall("class", ()).ok()?;
    let name: String = class_val.funcall("name", ()).ok()?;
    match name.as_str() {
        "TrueClass" => Some(true),
        "FalseClass" => Some(false),
        _ => None,
    }
}

fn hash_to_pairs(hash: RHash) -> Result<Vec<(MontyObject, MontyObject)>, Error> {
    let keys: RArray = hash.funcall("keys", ())?;
    let mut pairs = Vec::with_capacity(keys.len());
    for i in 0..keys.len() {
        let key: Value = keys.entry(i as isize)?;
        let val: Value = hash.aref(key)?;
        pairs.push((ruby_to_monty(key)?, ruby_to_monty(val)?));
    }
    Ok(pairs)
}

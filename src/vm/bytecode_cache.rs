// ==============================================================================
// AETHER v4.0 — Binary Bytecode Serializer & Precompiled `.aec` Cache
// Pure Rust Standard Library — Zero External Crates
// ==============================================================================

use std::sync::Arc;
use super::bytecode::Chunk;
use super::crypto::sha256_bytes;
use super::value::{CompiledFunction, Value};
use crate::syntax::span::Span;

const AEC_MAGIC: &[u8; 5] = b"AETH\x04";

pub fn serialize_compiled_function(
    func: &CompiledFunction,
    source_code: &str,
) -> Result<Vec<u8>, String> {
    let mut buf = Vec::with_capacity(512);
    buf.extend_from_slice(AEC_MAGIC);
    let src_hash = sha256_bytes(source_code.as_bytes());
    buf.extend_from_slice(&src_hash);
    write_function(&mut buf, func)?;
    Ok(buf)
}

pub fn deserialize_compiled_function(
    bytes: &[u8],
    expected_source: Option<&str>,
) -> Result<CompiledFunction, String> {
    if bytes.len() < 37 || &bytes[0..5] != AEC_MAGIC {
        return Err("Invalid or corrupted .aec bytecode header".to_string());
    }
    let stored_hash = &bytes[5..37];
    if let Some(src) = expected_source {
        let actual_hash = sha256_bytes(src.as_bytes());
        if stored_hash != actual_hash {
            return Err("Bytecode cache stale: source SHA-256 mismatch".to_string());
        }
    }
    let mut cursor = 37;
    read_function(bytes, &mut cursor)
}

fn write_u32(buf: &mut Vec<u8>, v: u32) {
    buf.extend_from_slice(&v.to_le_bytes());
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, String> {
    if *cursor + 4 > bytes.len() {
        return Err("Unexpected EOF reading u32 in .aec".to_string());
    }
    let v = u32::from_le_bytes([
        bytes[*cursor],
        bytes[*cursor + 1],
        bytes[*cursor + 2],
        bytes[*cursor + 3],
    ]);
    *cursor += 4;
    Ok(v)
}

fn write_string(buf: &mut Vec<u8>, s: &str) {
    let b = s.as_bytes();
    write_u32(buf, b.len() as u32);
    buf.extend_from_slice(b);
}

fn read_string(bytes: &[u8], cursor: &mut usize) -> Result<String, String> {
    let len = read_u32(bytes, cursor)? as usize;
    if *cursor + len > bytes.len() {
        return Err("Unexpected EOF reading string in .aec".to_string());
    }
    let s = std::str::from_utf8(&bytes[*cursor..*cursor + len])
        .map_err(|_| "Invalid UTF-8 string in .aec".to_string())?
        .to_string();
    *cursor += len;
    Ok(s)
}

fn write_function(buf: &mut Vec<u8>, func: &CompiledFunction) -> Result<(), String> {
    write_string(buf, &func.name);
    write_u32(buf, func.arity as u32);
    write_u32(buf, func.max_arity as u32);
    write_u32(buf, func.param_names.len() as u32);
    for p in &func.param_names {
        write_string(buf, p);
    }
    buf.push(u8::from(func.has_varargs));
    if let Some(ref vp) = func.varargs_param {
        buf.push(1);
        write_string(buf, vp);
    } else {
        buf.push(0);
    }
    buf.push(u8::from(func.has_kwargs));
    if let Some(ref kp) = func.kwargs_param {
        buf.push(1);
        write_string(buf, kp);
    } else {
        buf.push(0);
    }
    write_u32(buf, func.upvalue_count as u32);

    // Chunk code
    write_u32(buf, func.chunk.code.len() as u32);
    buf.extend_from_slice(&func.chunk.code);

    // Chunk constants
    write_u32(buf, func.chunk.constants.len() as u32);
    for c in &func.chunk.constants {
        write_constant(buf, c)?;
    }

    Ok(())
}

fn read_function(bytes: &[u8], cursor: &mut usize) -> Result<CompiledFunction, String> {
    let name = read_string(bytes, cursor)?;
    let arity = read_u32(bytes, cursor)? as usize;
    let max_arity = read_u32(bytes, cursor)? as usize;
    let param_count = read_u32(bytes, cursor)? as usize;
    let mut param_names = Vec::with_capacity(param_count);
    for _ in 0..param_count {
        param_names.push(read_string(bytes, cursor)?);
    }
    if *cursor >= bytes.len() {
        return Err("Unexpected EOF in function flags".to_string());
    }
    let has_varargs = bytes[*cursor] != 0;
    *cursor += 1;
    let has_vp = bytes[*cursor] != 0;
    *cursor += 1;
    let varargs_param = if has_vp {
        Some(read_string(bytes, cursor)?)
    } else {
        None
    };

    let has_kwargs = bytes[*cursor] != 0;
    *cursor += 1;
    let has_kp = bytes[*cursor] != 0;
    *cursor += 1;
    let kwargs_param = if has_kp {
        Some(read_string(bytes, cursor)?)
    } else {
        None
    };

    let upvalue_count = read_u32(bytes, cursor)? as usize;
    let code_len = read_u32(bytes, cursor)? as usize;
    if *cursor + code_len > bytes.len() {
        return Err("Unexpected EOF reading chunk code".to_string());
    }
    let code = bytes[*cursor..*cursor + code_len].to_vec();
    *cursor += code_len;

    let const_count = read_u32(bytes, cursor)? as usize;
    let mut constants = Vec::with_capacity(const_count);
    for _ in 0..const_count {
        constants.push(read_constant(bytes, cursor)?);
    }

    let spans = vec![Span::default(); code.len()];
    Ok(CompiledFunction {
        name,
        arity,
        max_arity,
        param_names,
        has_varargs,
        varargs_param,
        has_kwargs,
        kwargs_param,
        upvalue_count,
        chunk: Chunk {
            code,
            constants,
            spans,
        },
    })
}

fn write_constant(buf: &mut Vec<u8>, val: &Value) -> Result<(), String> {
    match val {
        Value::Nil => buf.push(0),
        Value::Bool(b) => {
            buf.push(1);
            buf.push(u8::from(*b));
        }
        Value::Int(i) => {
            buf.push(2);
            buf.extend_from_slice(&i.to_le_bytes());
        }
        Value::Float(f) => {
            buf.push(3);
            buf.extend_from_slice(&f.to_le_bytes());
        }
        Value::String(s) => {
            buf.push(4);
            write_string(buf, s);
        }
        Value::DefaultSentinel => buf.push(5),
        Value::Function(f) => {
            buf.push(6);
            write_function(buf, f)?;
        }
        other => {
            return Err(format!(
                "Cannot serialize non-literal constant of type '{}' in .aec",
                other.type_name()
            ))
        }
    }
    Ok(())
}

fn read_constant(bytes: &[u8], cursor: &mut usize) -> Result<Value, String> {
    if *cursor >= bytes.len() {
        return Err("Unexpected EOF reading constant tag".to_string());
    }
    let tag = bytes[*cursor];
    *cursor += 1;
    match tag {
        0 => Ok(Value::Nil),
        1 => {
            let b = bytes[*cursor] != 0;
            *cursor += 1;
            Ok(Value::Bool(b))
        }
        2 => {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&bytes[*cursor..*cursor + 8]);
            *cursor += 8;
            Ok(Value::Int(i64::from_le_bytes(arr)))
        }
        3 => {
            let mut arr = [0u8; 8];
            arr.copy_from_slice(&bytes[*cursor..*cursor + 8]);
            *cursor += 8;
            Ok(Value::Float(f64::from_le_bytes(arr)))
        }
        4 => Ok(Value::string(read_string(bytes, cursor)?)),
        5 => Ok(Value::DefaultSentinel),
        6 => Ok(Value::Function(Arc::new(read_function(bytes, cursor)?))),
        other => Err(format!("Unknown constant tag {} in .aec", other)),
    }
}

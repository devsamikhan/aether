#![allow(deprecated)]

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeValue {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    List(Vec<RuntimeValue>),
    Null,
}

impl IntoPy<PyObject> for RuntimeValue {
    fn into_py(self, py: Python<'_>) -> PyObject {
        match self {
            RuntimeValue::Integer(v) => v.into_py(py),
            RuntimeValue::Float(v) => v.into_py(py),
            RuntimeValue::String(v) => v.into_py(py),
            RuntimeValue::Boolean(v) => v.into_py(py),
            RuntimeValue::List(v) => {
                let list = PyList::empty(py);
                for item in v {
                    let val = item.into_py(py);
                    let _ = list.append(val);
                }
                list.into_py(py)
            }
            RuntimeValue::Null => py.None(),
        }
    }
}

impl<'source> FromPyObject<'source> for RuntimeValue {
    fn extract(ob: &'source PyAny) -> PyResult<Self> {
        if ob.is_none() {
            Ok(RuntimeValue::Null)
        } else if let Ok(b) = ob.downcast::<pyo3::types::PyBool>() {
            Ok(RuntimeValue::Boolean(b.is_true()))
        } else if let Ok(i) = ob.extract::<i64>() {
            Ok(RuntimeValue::Integer(i))
        } else if let Ok(f) = ob.extract::<f64>() {
            Ok(RuntimeValue::Float(f))
        } else if let Ok(s) = ob.extract::<String>() {
            Ok(RuntimeValue::String(s))
        } else if let Ok(lst) = ob.downcast::<pyo3::types::PyList>() {
            let mut v = Vec::new();
            for item in lst.iter() {
                v.push(Self::extract(item)?);
            }
            Ok(RuntimeValue::List(v))
        } else if let Ok(tup) = ob.downcast::<pyo3::types::PyTuple>() {
            let mut v = Vec::new();
            for item in tup.iter() {
                v.push(Self::extract(item)?);
            }
            Ok(RuntimeValue::List(v))
        } else {
            if let Ok(s) = ob.str() {
                Ok(RuntimeValue::String(s.to_string()))
            } else {
                Ok(RuntimeValue::Null)
            }
        }
    }
}

static MODULE_CACHE: OnceLock<Mutex<HashMap<String, PyObject>>> = OnceLock::new();

fn get_module_cache() -> &'static Mutex<HashMap<String, PyObject>> {
    MODULE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn get_py_gil<F, R>(f: F) -> R
where
    F: FnOnce(Python<'_>) -> R,
{
    Python::with_gil(f)
}

pub fn run_with_captured_output<F, R>(py: Python<'_>, f: F) -> Result<(R, String, String), String>
where
    F: FnOnce(Python<'_>) -> PyResult<R>,
{
    let sys = py
        .import("sys")
        .map_err(|e| format!("Failed to import sys: {:?}", e))?;
    let io = py
        .import("io")
        .map_err(|e| format!("Failed to import io: {:?}", e))?;

    let stdout_buf = io
        .call_method0("StringIO")
        .map_err(|e| format!("Failed to create StringIO: {:?}", e))?;
    let stderr_buf = io
        .call_method0("StringIO")
        .map_err(|e| format!("Failed to create StringIO: {:?}", e))?;

    let old_stdout = sys
        .getattr("stdout")
        .map_err(|e| format!("Failed to get stdout: {:?}", e))?
        .to_object(py);
    let old_stderr = sys
        .getattr("stderr")
        .map_err(|e| format!("Failed to get stderr: {:?}", e))?
        .to_object(py);

    sys.setattr("stdout", stdout_buf)
        .map_err(|e| format!("Failed to redirect stdout: {:?}", e))?;
    sys.setattr("stderr", stderr_buf)
        .map_err(|e| format!("Failed to redirect stderr: {:?}", e))?;

    let res = f(py);

    let restore_stdout_res = sys.setattr("stdout", &old_stdout);
    let restore_stderr_res = sys.setattr("stderr", &old_stderr);

    let stdout_str = stdout_buf
        .call_method0("getvalue")
        .and_then(|v| v.extract::<String>())
        .unwrap_or_default();
    let stderr_str = stderr_buf
        .call_method0("getvalue")
        .and_then(|v| v.extract::<String>())
        .unwrap_or_default();

    if let Err(e) = res {
        let err_msg = format!("Python Exception: {}\nStderr: {}", e, stderr_str);
        return Err(err_msg);
    }

    restore_stdout_res.map_err(|e| format!("Failed to restore stdout: {:?}", e))?;
    restore_stderr_res.map_err(|e| format!("Failed to restore stderr: {:?}", e))?;

    Ok((res.unwrap(), stdout_str, stderr_str))
}

pub fn py_import(module_name: &str) -> Result<RuntimeValue, String> {
    let cache = get_module_cache();
    {
        let guard = cache.lock().unwrap();
        if guard.contains_key(module_name) {
            return Ok(RuntimeValue::Null);
        }
    }

    let res: PyResult<()> = get_py_gil(|py| {
        let m = py.import(module_name)?;
        let mut guard = cache.lock().unwrap();
        guard.insert(module_name.to_string(), m.to_object(py));
        Ok(())
    });

    match res {
        Ok(()) => Ok(RuntimeValue::Null),
        Err(e) => Err(format!(
            "E0401: Failed to import Python module '{}': {}",
            module_name, e
        )),
    }
}

pub fn py_eval(expr: &str) -> Result<RuntimeValue, String> {
    get_py_gil(|py| {
        let globals = PyDict::new(py);
        {
            let cache = get_module_cache().lock().unwrap();
            for (name, obj) in cache.iter() {
                globals.set_item(name, obj).ok();
            }
        }
        if let Ok(builtins) = py.import("builtins") {
            globals.set_item("__builtins__", builtins).ok();
        }

        let run_res = run_with_captured_output(py, |py| {
            py.eval(expr, Some(globals), None)?
                .extract::<RuntimeValue>()
        });

        match run_res {
            Ok((val, stdout, stderr)) => {
                if !stdout.is_empty() {
                    print!("{}", stdout);
                }
                if !stderr.is_empty() {
                    eprint!("{}", stderr);
                }
                Ok(val)
            }
            Err(e) => Err(format!("E0400: Python Interop Error during eval: {}", e)),
        }
    })
}

pub fn py_exec(code: &str) -> Result<RuntimeValue, String> {
    get_py_gil(|py| {
        let globals = PyDict::new(py);
        {
            let cache = get_module_cache().lock().unwrap();
            for (name, obj) in cache.iter() {
                globals.set_item(name, obj).ok();
            }
        }
        if let Ok(builtins) = py.import("builtins") {
            globals.set_item("__builtins__", builtins).ok();
        }

        let indented_code: String = code
            .lines()
            .map(|line| format!("    {}", line))
            .collect::<Vec<String>>()
            .join("\n");

        let wrapped_code = format!("def __aether_tmp_func__():\n{}\n", indented_code);

        let run_res = run_with_captured_output(py, |py| {
            py.run(&wrapped_code, Some(globals), None)?;
            let func = globals.get_item("__aether_tmp_func__")?.ok_or_else(|| {
                pyo3::exceptions::PyRuntimeError::new_err("Temp function not found")
            })?;
            func.call0()?.extract::<RuntimeValue>()
        });

        match run_res {
            Ok((val, stdout, stderr)) => {
                if !stdout.is_empty() {
                    print!("{}", stdout);
                }
                if !stderr.is_empty() {
                    eprint!("{}", stderr);
                }
                Ok(val)
            }
            Err(e) => Err(format!("E0400: Python Interop Error during exec: {}", e)),
        }
    })
}

pub fn py_call(func_name: &str, args: Vec<RuntimeValue>) -> Result<RuntimeValue, String> {
    get_py_gil(|py| {
        let run_res = run_with_captured_output(py, |py| {
            let parts: Vec<&str> = func_name.split('.').collect();
            if parts.is_empty() {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "Empty function name",
                ));
            }

            let obj: PyObject = if parts.len() == 1 {
                let builtins = py.import("builtins")?;
                if let Ok(item) = builtins.getattr(parts[0]) {
                    item.to_object(py)
                } else {
                    let cache = get_module_cache().lock().unwrap();
                    if let Some(cached) = cache.get(parts[0]) {
                        cached.clone()
                    } else {
                        return Err(pyo3::exceptions::PyNameError::new_err(format!(
                            "Name '{}' not found",
                            parts[0]
                        )));
                    }
                }
            } else {
                let mut current = {
                    let cache = get_module_cache().lock().unwrap();
                    if let Some(cached) = cache.get(parts[0]) {
                        cached.clone()
                    } else {
                        py.import(parts[0])?.to_object(py)
                    }
                };

                for part in parts.iter().skip(1) {
                    current = current.getattr(py, *part)?;
                }
                current
            };

            let py_args = PyTuple::new(py, args.into_iter().map(|arg| arg.into_py(py)));
            let res = obj.call1(py, py_args)?;
            res.extract::<RuntimeValue>(py)
        });

        match run_res {
            Ok((val, stdout, stderr)) => {
                if !stdout.is_empty() {
                    print!("{}", stdout);
                }
                if !stderr.is_empty() {
                    eprint!("{}", stderr);
                }
                Ok(val)
            }
            Err(e) => Err(format!("E0400: Python Interop Error during call: {}", e)),
        }
    })
}

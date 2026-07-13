use aether::interop::python::{py_call, py_eval, py_exec, py_import, RuntimeValue};
use pyo3::IntoPy;

fn is_python_available() -> bool {
    std::panic::catch_unwind(|| pyo3::Python::with_gil(|_py| true)).unwrap_or(false)
}

#[test]
fn test_runtime_value_conversions() {
    if !is_python_available() {
        println!("Python not available, skipping test_runtime_value_conversions.");
        return;
    }

    pyo3::Python::with_gil(|py| {
        // Test Integer
        let val = RuntimeValue::Integer(42);
        let py_obj = val.into_py(py);
        let back: RuntimeValue = py_obj.extract(py).unwrap();
        assert_eq!(back, RuntimeValue::Integer(42));

        // Test Float
        let val = RuntimeValue::Float(3.14);
        let py_obj = val.into_py(py);
        let back: RuntimeValue = py_obj.extract(py).unwrap();
        assert_eq!(back, RuntimeValue::Float(3.14));

        // Test String
        let val = RuntimeValue::String("hello aether".to_string());
        let py_obj = val.into_py(py);
        let back: RuntimeValue = py_obj.extract(py).unwrap();
        assert_eq!(back, RuntimeValue::String("hello aether".to_string()));

        // Test Boolean
        let val = RuntimeValue::Boolean(true);
        let py_obj = val.into_py(py);
        let back: RuntimeValue = py_obj.extract(py).unwrap();
        assert_eq!(back, RuntimeValue::Boolean(true));

        // Test Null
        let val = RuntimeValue::Null;
        let py_obj = val.into_py(py);
        let back: RuntimeValue = py_obj.extract(py).unwrap();
        assert_eq!(back, RuntimeValue::Null);

        // Test List
        let val = RuntimeValue::List(vec![
            RuntimeValue::Integer(1),
            RuntimeValue::Float(2.5),
            RuntimeValue::String("three".to_string()),
        ]);
        let py_obj = val.into_py(py);
        let back: RuntimeValue = py_obj.extract(py).unwrap();
        assert_eq!(
            back,
            RuntimeValue::List(vec![
                RuntimeValue::Integer(1),
                RuntimeValue::Float(2.5),
                RuntimeValue::String("three".to_string()),
            ])
        );
    });
}

#[test]
fn test_python_eval() {
    if !is_python_available() {
        println!("Python not available, skipping test_python_eval.");
        return;
    }

    let res = py_eval("1 + 1").unwrap();
    assert_eq!(res, RuntimeValue::Integer(2));

    let res = py_eval("'hello'.upper()").unwrap();
    assert_eq!(res, RuntimeValue::String("HELLO".to_string()));
}

#[test]
fn test_python_exec() {
    if !is_python_available() {
        println!("Python not available, skipping test_python_exec.");
        return;
    }

    let code = r#"
a = 10
b = 20
return a + b
"#;
    let res = py_exec(code).unwrap();
    assert_eq!(res, RuntimeValue::Integer(30));
}

#[test]
fn test_python_import_and_call() {
    if !is_python_available() {
        println!("Python not available, skipping test_python_import_and_call.");
        return;
    }

    let import_res = py_import("math").unwrap();
    assert_eq!(import_res, RuntimeValue::Null);

    let call_res = py_call("math.sqrt", vec![RuntimeValue::Integer(16)]).unwrap();
    assert_eq!(call_res, RuntimeValue::Float(4.0));
}

#[test]
fn test_python_error_handling() {
    if !is_python_available() {
        println!("Python not available, skipping test_python_error_handling.");
        return;
    }

    let res = py_import("non_existent_module_xyz_123");
    assert!(res.is_err());
    let err = res.err().unwrap();
    assert!(err.contains("E0401"));

    let res = py_eval("1 +");
    assert!(res.is_err());
    let err = res.err().unwrap();
    assert!(err.contains("E0400"));
}

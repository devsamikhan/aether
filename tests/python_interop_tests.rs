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

#[test]
fn test_aether_data_science_pipeline() {
    if !is_python_available() {
        println!("Python not available, skipping test_aether_data_science_pipeline.");
        return;
    }

    let code = r##"
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt

# 1. Read / Create CSV with Pandas
data = {
    'Product': ['Alpha', 'Beta', 'Gamma', 'Delta', 'Epsilon'],
    'Sales': [12000, 15000, 9000, 22000, 18000],
    'Quantity': [120, 150, 95, 210, 190]
}
df = pd.DataFrame(data)
df.to_csv('data.csv', index=False)

# 2. Analyze with NumPy
sales_arr = df['Sales'].to_numpy()
avg_sales = np.mean(sales_arr)
max_sales = np.max(sales_arr)
total_qty = np.sum(df['Quantity'].to_numpy())

# 3. Visualize with Matplotlib
plt.figure(figsize=(8, 5))
plt.bar(df['Product'], df['Sales'], color='purple')
plt.title('Product Sales Performance')
plt.xlabel('Product')
plt.ylabel('Sales ($)')
plt.grid(True, linestyle='--', alpha=0.6)
plt.savefig('sales_plot.png')
plt.close()

# 4. Generate Reports
report = f"""# AETHER Data Science Report
Generated via GIL-safe Python FFI bridge.

## Key Metrics:
- **Total Products Analyzed**: {len(df)}
- **Average Sales**: ${avg_sales:,.2f}
- **Maximum Sales**: ${max_sales:,.2f}
- **Total Units Sold**: {total_qty}

## Visualization
Product sales distribution has been plotted and saved to `sales_plot.png`.
"""
with open('report.md', 'w') as f:
    f.write(report)

return avg_sales
"##;

    let res = py_exec(code).unwrap();
    assert_eq!(res, RuntimeValue::Float(15200.0));

    // Verify report and plot files exist
    assert!(std::path::Path::new("data.csv").exists());
    assert!(std::path::Path::new("sales_plot.png").exists());
    assert!(std::path::Path::new("report.md").exists());

    // Clean up files
    let _ = std::fs::remove_file("data.csv");
    let _ = std::fs::remove_file("sales_plot.png");
    let _ = std::fs::remove_file("report.md");
}

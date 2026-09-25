use aether::vm::{run_source, Value};

#[test]
fn test_aether_tensor_native_creation_and_basics() {
    let code = r#"
z = matrix.zeros([2, 3])
o = matrix.ones([2, 2])
e = matrix.eye(2)
arr = matrix.from_array([[1.0, 2.0], [3.0, 4.0]])

[z["shape"], o["shape"], e["shape"], matrix.to_list(arr)]
"#;
    let res = run_source(code).expect("Tensor creation and basics failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        // z shape: [2, 3]
        if let Value::Array(z_shape) = &items[0] {
            assert_eq!(z_shape.lock().clone(), vec![Value::Int(2), Value::Int(3)]);
        } else {
            panic!("Expected array shape");
        }
        // e shape: [2, 2]
        if let Value::Array(e_shape) = &items[2] {
            assert_eq!(e_shape.lock().clone(), vec![Value::Int(2), Value::Int(2)]);
        } else {
            panic!("Expected array shape");
        }
    } else {
        panic!("Expected Array result, got {:?}", res);
    }
}

#[test]
fn test_aether_tensor_matmul() {
    let code = r#"
a = matrix.from_array([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]])
b = matrix.from_array([[7.0, 8.0], [9.0, 1.0], [2.0, 3.0]])

c = matrix.matmul(a, b)
matrix.to_list(c)
"#;
    let res = run_source(code).expect("Tensor MatMul failed");
    if let Value::Array(rows) = res {
        let r_items = rows.lock().clone();
        assert_eq!(r_items.len(), 2);
        if let Value::Array(r0) = &r_items[0] {
            let row0 = r0.lock().clone();
            assert_eq!(row0[0], Value::Float(31.0));
            assert_eq!(row0[1], Value::Float(19.0));
        }
        if let Value::Array(r1) = &r_items[1] {
            let row1 = r1.lock().clone();
            assert_eq!(row1[0], Value::Float(85.0));
            assert_eq!(row1[1], Value::Float(55.0));
        }
    } else {
        panic!("Expected 2D array, got {:?}", res);
    }
}

#[test]
fn test_aether_tensor_broadcasting_and_arithmetic() {
    let code = r#"
a = matrix.from_array([[1.0, 2.0], [3.0, 4.0]])
# Scalar broadcast addition
b = matrix.add(a, 10.0)
# Element-wise multiply
c = matrix.mul(a, 2.0)

[matrix.to_list(b), matrix.to_list(c)]
"#;
    let res = run_source(code).expect("Tensor broadcasting failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        if let Value::Array(b_rows) = &items[0] {
            let r0 = b_rows.lock()[0].clone();
            if let Value::Array(r0_arr) = r0 {
                assert_eq!(r0_arr.lock().clone(), vec![Value::Float(11.0), Value::Float(12.0)]);
            }
        }
        if let Value::Array(c_rows) = &items[1] {
            let r0 = c_rows.lock()[0].clone();
            if let Value::Array(r0_arr) = r0 {
                assert_eq!(r0_arr.lock().clone(), vec![Value::Float(2.0), Value::Float(4.0)]);
            }
        }
    } else {
        panic!("Expected Array result, got {:?}", res);
    }
}

#[test]
fn test_aether_tensor_activations_and_reductions() {
    let code = r#"
x = matrix.from_array([-2.0, -0.5, 0.0, 1.5, 3.0])
r = matrix.relu(x)
s = matrix.sum(r)
m = matrix.mean(r)

zero_t = matrix.from_array([0.0])
sig_zero = matrix.sigmoid(zero_t)

sm = matrix.softmax(x)
sm_sum = matrix.sum(sm)

[matrix.to_list(r), s, m, matrix.to_list(sig_zero)[0], sm_sum]
"#;
    let res = run_source(code).expect("Tensor activations and reductions failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        // relu: [0.0, 0.0, 0.0, 1.5, 3.0]
        if let Value::Array(r_arr) = &items[0] {
            assert_eq!(r_arr.lock().clone(), vec![
                Value::Float(0.0),
                Value::Float(0.0),
                Value::Float(0.0),
                Value::Float(1.5),
                Value::Float(3.0)
            ]);
        }
        // sum: 4.5
        assert_eq!(items[1], Value::Float(4.5));
        // mean: 4.5 / 5 = 0.9
        assert_eq!(items[2], Value::Float(0.9));
        // sigmoid(0) = 0.5
        assert_eq!(items[3], Value::Float(0.5));
        // softmax sum ≈ 1.0
        if let Value::Float(val) = items[4] {
            assert!((val - 1.0).abs() < 1e-5);
        } else {
            panic!("Expected float softmax sum");
        }
    } else {
        panic!("Expected Array result, got {:?}", res);
    }
}

#[test]
fn test_aether_tensor_oop_class() {
    let code = r#"
from aether_tensor import Tensor

t1 = Tensor([[1.0, 2.0], [3.0, 4.0]])
t2 = Tensor([[5.0, 6.0], [7.0, 8.0]])

# Operator overloading __add__
t_add = t1 + t2

# Matmul
t_mm = t1.matmul(t2)

# Transpose
t_trans = t1.transpose()

[t_add.to_list(), t_mm.to_list(), t_trans.to_list()]
"#;
    let res = run_source(code).expect("Tensor OOP class failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        // t_add: [[6.0, 8.0], [10.0, 12.0]]
        if let Value::Array(r0) = &items[0] {
            let row0 = r0.lock()[0].clone();
            if let Value::Array(r0_vals) = row0 {
                assert_eq!(r0_vals.lock().clone(), vec![Value::Float(6.0), Value::Float(8.0)]);
            }
        }
        // t_trans: [[1.0, 3.0], [2.0, 4.0]]
        if let Value::Array(r2) = &items[2] {
            let row0 = r2.lock()[0].clone();
            if let Value::Array(r0_vals) = row0 {
                assert_eq!(r0_vals.lock().clone(), vec![Value::Float(1.0), Value::Float(3.0)]);
            }
        }
    } else {
        panic!("Expected Array result, got {:?}", res);
    }
}

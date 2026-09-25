use aether::vm::{run_source, Value};

#[test]
fn test_autograd_basic_graph_and_gradients() {
    let code = r#"
from aether_autograd import Variable

# f(x, y) = x * y + x
# df/dx = y + 1
# df/dy = x
x = Variable([[2.0, 3.0]])
y = Variable([[4.0, 5.0]])

prod = x * y
f = prod + x
loss = f.sum()
loss.backward()

# Expected x.grad: [[4.0 + 1.0, 5.0 + 1.0]] = [[5.0, 6.0]]
# Expected y.grad: [[2.0, 3.0]]
[
    x.grad.to_list(),
    y.grad.to_list(),
    loss.data.to_list()
]
"#;
    let res = run_source(code).expect("Autograd basic graph test failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        // x.grad
        assert_eq!(items[0].to_string(), "[[5, 6]]");
        // y.grad
        assert_eq!(items[1].to_string(), "[[2, 3]]");
        // loss: 2*4 + 2 + 3*5 + 3 = 8 + 2 + 15 + 3 = 28
        assert_eq!(items[2].to_string(), "[28]");
    } else {
        panic!("Expected Array, got {:?}", res);
    }
}

#[test]
fn test_autograd_matmul_and_activation() {
    let code = r#"
from aether_autograd import Variable

# X: [1, 2], W: [2, 2]
X = Variable([[1.0, -2.0]])
W = Variable([[0.5, -0.5], [1.5, 2.0]])

# Z = X @ W = [[1*0.5 + -2*1.5, 1*-0.5 + -2*2.0]] = [[-2.5, -4.5]]
Z = X.matmul(W)
# Relu(Z) = [[0.0, 0.0]]
a = Z.relu()
loss = a.sum()
loss.backward()

# Since Z elements are both negative, ReLU mask is 0, so all grads are 0
[
    X.grad.to_list(),
    W.grad.to_list()
]
"#;
    let res = run_source(code).expect("Autograd matmul & activation test failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        assert_eq!(items[0].to_string(), "[[0, 0]]");
        assert_eq!(items[1].to_string(), "[[0, 0], [0, 0]]");
    } else {
        panic!("Expected Array, got {:?}", res);
    }
}

#[test]
fn test_neural_linear_layer_optimization_loop() {
    let code = r#"
from aether_tensor import Tensor
from aether_autograd import Variable, Linear, SGD, mse_loss

# Target function: y = X @ [[2.0], [-1.0]] + 0.5
X = Variable([[1.0, 2.0], [2.0, 1.0], [0.0, 1.0], [3.0, 0.0]])
target = Variable([[0.5], [3.5], [-0.5], [6.5]])

model = Linear(2, 1, true)
opt = SGD(model.parameters(), 0.05)

initial_loss = 0.0
final_loss = 0.0

epoch = 0
while epoch < 40:
    opt.zero_grad()
    pred = model.forward(X)
    loss = mse_loss(pred, target)
    if epoch == 0:
        initial_loss = loss.data.to_list()[0]
    final_loss = loss.data.to_list()[0]
    loss.backward()
    opt.step()
    epoch = epoch + 1

[initial_loss, final_loss, final_loss < initial_loss]
"#;
    let res = run_source(code).expect("Neural network optimization loop failed");
    if let Value::Array(arr) = res {
        let items = arr.lock().clone();
        let initial_loss = match items[0] {
            Value::Float(f) => f,
            Value::Int(i) => i as f64,
            _ => panic!("Expected numeric initial_loss"),
        };
        let final_loss = match items[1] {
            Value::Float(f) => f,
            Value::Int(i) => i as f64,
            _ => panic!("Expected numeric final_loss"),
        };
        let decreased = match items[2] {
            Value::Bool(b) => b,
            _ => false,
        };
        assert!(decreased, "Loss should have decreased: initial={}, final={}", initial_loss, final_loss);
        assert!(final_loss < initial_loss / 2.0, "Loss should reduce by at least 50%: initial={}, final={}", initial_loss, final_loss);
    } else {
        panic!("Expected Array, got {:?}", res);
    }
}

use aether::vm::{run_source, Value};
use tempfile::tempdir;

#[test]
fn test_tensor_numpy_math_axis_and_slicing_parity() {
    let code = r#"
from aether_tensor import Tensor

let r = Tensor.arange(0.0, 5.0, 1.0)
let ls = Tensor.linspace(0.0, 10.0, 5)
let m = Tensor([[1.0, 4.0, 2.0], [3.0, 0.0, 9.0]])

let sum_cols = m.sum_axis(0)
let mean_rows = m.mean_axis(1)
let argmax_rows = m.argmax(1)
let g = Tensor([-1.0, 0.0, 2.0]).gelu()
let e = Tensor([0.0, 1.0]).exp()
let l = e.log()
let sq = Tensor([4.0, 9.0]).sqrt()
let sl = m.slice_rows(1, 2)

[
    r.to_list(),
    ls.to_list(),
    sum_cols.to_list(),
    mean_rows.to_list(),
    argmax_rows.to_list(),
    m.min(),
    m.max(),
    m.argmax(),
    m.argmin(),
    l.to_list(),
    sq.to_list(),
    sl.to_list(),
    g.shape()
]
"#;
    let res = run_source(code).expect("Extended tensor operations failed");
    let Value::Array(items) = res else {
        panic!("Expected array result, got {:?}", res);
    };
    let v = items.lock().clone();

    // arange: [0, 1, 2, 3, 4]
    assert_eq!(format!("{}", v[0]), "[0, 1, 2, 3, 4]");
    // linspace: [0, 2.5, 5, 7.5, 10]
    assert_eq!(format!("{}", v[1]), "[0, 2.5, 5, 7.5, 10]");
    // sum_axis(0): [[4, 4, 11]]
    assert_eq!(format!("{}", v[2]), "[[4, 4, 11]]");
    // argmax_axis(1): [1, 2]
    assert_eq!(format!("{}", v[4]), "[1, 2]");
    assert_eq!(v[5], Value::Float(0.0));
    assert_eq!(v[6], Value::Float(9.0));
    assert_eq!(v[7], Value::Int(5)); // max 9.0 at flat index 5
    assert_eq!(v[8], Value::Int(4)); // min 0.0 at flat index 4
                                     // sqrt([4, 9]) = [2, 3]
    assert_eq!(format!("{}", v[10]), "[2, 3]");
    // slice_rows(1, 2) = [[3, 0, 9]]
    assert_eq!(format!("{}", v[11]), "[[3, 0, 9]]");
    assert_eq!(format!("{}", v[12]), "[3]");
}

#[test]
fn test_ml_linear_logistic_and_metrics_bypass_sklearn() {
    let code = r#"
from aether_ml import StandardScaler, LinearRegression, LogisticRegression, train_test_split

# 1. Exact Multivariate Linear Regression: y = 3*x1 - 2*x2 + 5
let X_lin = [
    [1.0, 0.0],
    [0.0, 1.0],
    [2.0, 1.0],
    [3.0, 2.0],
    [-1.0, 4.0],
    [5.0, -2.0]
]
let y_lin = [8.0, 3.0, 9.0, 10.0, -6.0, 24.0]

let lin = LinearRegression().fit(X_lin, y_lin)
let preds_lin = lin.predict([[4.0, 1.0], [0.0, 0.0]])
let r2 = lin.score(X_lin, y_lin)

# 2. Logistic Regression + StandardScaler + Split + ROC-AUC
let X_cls = [
    [10.0, 1.0], [12.0, 2.0], [11.0, 0.5], [14.0, 1.5], [13.0, 3.0],
    [1.0, 10.0], [2.0, 11.0], [0.5, 13.0], [3.0, 12.0], [1.5, 9.5]
]
let y_cls = [1, 1, 1, 1, 1, 0, 0, 0, 0, 0]

let split = train_test_split(X_cls, y_cls, 0.3, 42)
let scaler = StandardScaler()
let X_tr = scaler.fit_transform(split["X_train"])
let X_te = scaler.transform(split["X_test"])

let log_reg = LogisticRegression(0.5, 300, 0.001).fit(X_tr, split["y_train"])
let acc = log_reg.score(X_te, split["y_test"])
let proba = log_reg.predict_proba(X_te)
let pos_scores = []
for row in proba:
    pos_scores.push(row[1])
let auc = ML.roc_auc(y_cls, [0.95, 0.92, 0.91, 0.99, 0.88, 0.1, 0.05, 0.02, 0.12, 0.08])
let prf = ML.precision_recall_f1([1, 1, 0, 0], [1, 1, 0, 0], 1)

[preds_lin[0], preds_lin[1], r2, acc, auc, prf["f1"]]
"#;

    let res = run_source(code).expect("Linear/Logistic ML test failed");
    let Value::Array(items) = res else {
        panic!("Expected Array result, got {:?}", res);
    };
    let v = items.lock().clone();
    let p0 = match v[0] {
        Value::Float(f) => f,
        _ => panic!("Expected float"),
    };
    let p1 = match v[1] {
        Value::Float(f) => f,
        _ => panic!("Expected float"),
    };
    let r2 = match v[2] {
        Value::Float(f) => f,
        _ => panic!("Expected float"),
    };
    let acc = match v[3] {
        Value::Float(f) => f,
        _ => panic!("Expected float"),
    };
    let auc = match v[4] {
        Value::Float(f) => f,
        _ => panic!("Expected float"),
    };
    let f1 = match v[5] {
        Value::Float(f) => f,
        _ => panic!("Expected float"),
    };

    assert!(
        (p0 - 15.0).abs() < 1e-3,
        "Expected 3*4 - 2*1 + 5 = 15, got {}",
        p0
    );
    assert!((p1 - 5.0).abs() < 1e-3, "Expected bias 5.0, got {}", p1);
    assert!(r2 > 0.9999, "Expected R2 > 0.9999, got {}", r2);
    assert!(acc >= 0.99, "Expected accuracy >= 0.99, got {}", acc);
    assert!(
        (auc - 1.0).abs() < 1e-6,
        "Expected ROC-AUC 1.0, got {}",
        auc
    );
    assert!((f1 - 1.0).abs() < 1e-6, "Expected F1 1.0, got {}", f1);
}

#[test]
fn test_ml_random_forest_kmeans_pca_and_model_persistence() {
    let dir = tempdir().expect("Failed to create tempdir");
    let model_path = dir.path().join("rf_edge_model.json");
    let model_path_str = model_path.to_string_lossy().replace('\\', "/");

    let code = format!(
        r#"
from aether_ml import DecisionTree, RandomForest, KNN, KMeans, PCA, load_model

# Non-linear XOR-like dataset
let X = [
    [0.1, 0.1], [0.2, 0.2], [0.15, 0.3], [0.25, 0.15],
    [0.85, 0.85], [0.9, 0.9], [0.8, 0.75], [0.95, 0.8],
    [0.1, 0.9], [0.2, 0.85], [0.15, 0.8], [0.25, 0.95],
    [0.9, 0.1], [0.85, 0.2], [0.8, 0.15], [0.95, 0.25]
]
let y = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1]

let dt = DecisionTree(5, 2).fit(X, y)
let rf = RandomForest(21, 5, 77).fit(X, y)
let knn = KNN(3).fit(X, y)

# Save RandomForest to disk and reload in same runtime
let saved = rf.save("{path}")
let reloaded_id = load_model("{path}")
let reloaded_preds = ML.predict(reloaded_id, [[0.12, 0.18], [0.12, 0.88], [0.88, 0.12], [0.88, 0.88]])
let cm = ML.confusion_matrix(y, rf.predict(X))

# KMeans on 2 tight clusters
let km = KMeans(2, 50, 42).fit([[0.0, 0.0], [0.1, 0.1], [10.0, 10.0], [10.1, 10.1]])
let clusters = km.predict([[0.05, 0.05], [9.9, 9.9]])

# PCA 3D -> 2D
let X_3d = [[1.0, 2.0, 0.01], [2.0, 4.0, 0.02], [3.0, 6.0, 0.01], [4.0, 8.0, 0.03]]
let pca = PCA(2).fit(X_3d)
let proj = pca.transform(X_3d)
let var = pca.explained_variance()

[
    dt.score(X, y),
    rf.score(X, y),
    knn.score(X, y),
    saved,
    reloaded_preds,
    clusters[0] != clusters[1],
    len(proj),
    len(proj[0]),
    var[0] > var[1],
    cm["labels"]
]
"#,
        path = model_path_str
    );

    let res = run_source(&code).expect("RF/KMeans/PCA test failed");
    let Value::Array(items) = res else {
        panic!("Expected Array result, got {:?}", res);
    };
    let v = items.lock().clone();
    assert_eq!(v[0], Value::Float(1.0)); // DecisionTree XOR accuracy = 1.0
    assert!(
        matches!(v[1], Value::Float(f) if f >= 0.90),
        "RF accuracy {:?}",
        v[1]
    );
    assert_eq!(v[2], Value::Float(1.0)); // KNN XOR accuracy = 1.0
    assert_eq!(v[3], Value::Bool(true)); // saved
    assert_eq!(format!("{}", v[4]), "[0, 1, 1, 0]"); // reloaded model XOR predictions
    assert_eq!(v[5], Value::Bool(true)); // KMeans separated clusters
    assert_eq!(v[6], Value::Int(4)); // PCA rows
    assert_eq!(v[7], Value::Int(2)); // PCA n_components=2
    assert_eq!(v[8], Value::Bool(true)); // first PC captures dominant variance
    assert_eq!(format!("{}", v[9]), "[0, 1]");
}

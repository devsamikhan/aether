use aether::vm::{run_source, Value};

#[test]
fn test_aethercache_operations() {
    let code = r#"
class CacheItem:
    def __init__(self, key, value):
        self.key = key
        self.value = value

class MiniCache:
    def __init__(self, capacity=10):
        self.capacity = capacity
        self.data = {}
        self.hits = 0
        self.misses = 0

    def set(self, key, val):
        self.data[str(key)] = val

    def get(self, key, default=nil):
        k = str(key)
        if contains(self.data, k):
            self.hits += 1
            return self.data[k]
        self.misses += 1
        return default

    def __len__(self):
        return len(self.data)

    def __getitem__(self, key):
        return self.get(key, nil)

    def __setitem__(self, key, val):
        self.set(key, val)

cache = MiniCache(capacity=5)
cache["user:1"] = "Alice"
cache["user:2"] = "Bob"
cache.set("user:3", "Charlie")

val1 = cache["user:1"]
val2 = cache.get("user:2")
val3 = cache["user:3"]
missing = cache.get("user:99", "DEFAULT")

[len(cache), val1, val2, val3, missing, cache.hits, cache.misses]
"#;
    let result = run_source(code).expect("MiniCache operations failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(3));
        assert_eq!(items[1], Value::string("Alice"));
        assert_eq!(items[2], Value::string("Bob"));
        assert_eq!(items[3], Value::string("Charlie"));
        assert_eq!(items[4], Value::string("DEFAULT"));
        assert_eq!(items[5], Value::Int(3)); // 3 hits
        assert_eq!(items[6], Value::Int(1)); // 1 miss
    } else {
        panic!("Expected array of results");
    }
}

#[test]
fn test_aethercache_lru_eviction() {
    let code = r#"
class EvictingCache:
    def __init__(self, capacity=3):
        self.capacity = capacity
        self.store = {}
        self.order = []

    def set(self, k, v):
        key = str(k)
        if len(self.store) >= self.capacity and not contains(self.store, key):
            oldest = self.order[0]
            remove(self.store, oldest)
            remove(self.order, oldest)
        self.store[key] = v
        if not contains(self.order, key):
            append(self.order, key)

    def get(self, k, default="MISS"):
        key = str(k)
        if contains(self.store, key):
            return self.store[key]
        return default

c = EvictingCache(capacity=2)
c.set("k1", 100)
c.set("k2", 200)
c.set("k3", 300) # Should evict k1

[c.get("k1"), c.get("k2"), c.get("k3")]
"#;
    let result = run_source(code).expect("LRU eviction failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::string("MISS")); // k1 was evicted
        assert_eq!(items[1], Value::Int(200));
        assert_eq!(items[2], Value::Int(300));
    } else {
        panic!("Expected array of results");
    }
}

#[test]
fn test_neural_network_matrix_multiplication() {
    let code = r#"
class Matrix2x2:
    def __init__(self, a, b, c, d):
        self.a = a
        self.b = b
        self.c = c
        self.d = d

    def __mul__(self, other):
        # [a b] * [oa ob] = [a*oa + b*oc, a*ob + b*od]
        # [c d]   [oc od]   [c*oa + d*oc, c*ob + d*od]
        new_a = self.a * other.a + self.b * other.c
        new_b = self.a * other.b + self.b * other.d
        new_c = self.c * other.a + self.d * other.c
        new_d = self.c * other.b + self.d * other.d
        return Matrix2x2(new_a, new_b, new_c, new_d)

    def to_list(self):
        return [self.a, self.b, self.c, self.d]

# Identity * Matrix
m1 = Matrix2x2(1, 2, 3, 4)
m2 = Matrix2x2(2, 0, 1, 2)
# [1 2] * [2 0] = [1*2 + 2*1, 1*0 + 2*2] = [4, 4]
# [3 4]   [1 2]   [3*2 + 4*1, 3*0 + 4*2] = [10, 8]
m3 = m1 * m2
m3.to_list()
"#;
    let result = run_source(code).expect("Matrix multiplication failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(4));
        assert_eq!(items[1], Value::Int(4));
        assert_eq!(items[2], Value::Int(10));
        assert_eq!(items[3], Value::Int(8));
    } else {
        panic!("Expected array of results");
    }
}

#[test]
fn test_neural_network_forward_inference() {
    let code = r#"
def relu(x):
    if x > 0.0:
        return x
    return 0.0

class Neuron:
    def __init__(self, w1, w2, bias):
        self.w1 = w1
        self.w2 = w2
        self.bias = bias

    def forward(self, x1, x2):
        z = (x1 * self.w1) + (x2 * self.w2) + self.bias
        return relu(z)

class TwoNeuronLayer:
    def __init__(self):
        self.n1 = Neuron(0.5, -0.5, 0.1)
        self.n2 = Neuron(1.0, 2.0, -1.0)

    def forward(self, x1, x2):
        out1 = self.n1.forward(x1, x2)
        out2 = self.n2.forward(x1, x2)
        return [out1, out2]

layer = TwoNeuronLayer()
# x1=2.0, x2=1.0:
# n1: 2*0.5 + 1*(-0.5) + 0.1 = 1 - 0.5 + 0.1 = 0.6 -> relu(0.6) = 0.6
# n2: 2*1.0 + 1*2.0 - 1.0 = 2 + 2 - 1 = 3.0 -> relu(3.0) = 3.0
out = layer.forward(2.0, 1.0)
out
"#;
    let result = run_source(code).expect("Neural inference failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        if let Value::Float(f1) = items[0] {
            assert!((f1 - 0.6).abs() < 1e-6);
        } else {
            panic!("Expected float output for n1");
        }
        if let Value::Float(f2) = items[1] {
            assert!((f2 - 3.0).abs() < 1e-6);
        } else {
            panic!("Expected float output for n2");
        }
    } else {
        panic!("Expected array of results");
    }
}

const samples = {
  hello: {
    code: `// 01. Hello World & Basic Arithmetic
let greeting = "Hello, Developer!";
let a = 15;
let b = 27;
let sum = a + b;

println(greeting);
println("Sum of " + to_string(a) + " + " + to_string(b) + " = " + to_string(sum));`,
    logs: `[AETHER VM] Parsing zero-ceremony script...
Hello, Developer!
Sum of 15 + 27 = 42
[Execution finished in 0.8ms - Zero GC Pause]`
  },
  lists: {
    code: `// 02. Lists & Python-Grade Comprehensions
let numbers = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

// Square all even numbers concisely
let even_squares = [x * x for x in numbers if x % 2 == 0];

println("Original: " + to_string(numbers));
println("Even Squares: " + to_string(even_squares));`,
    logs: `[AETHER VM] Allocated contiguous dynamic list...
Original: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
Even Squares: [4, 16, 36, 64, 100]
[Execution finished in 1.1ms]`
  },
  functions: {
    code: `// 03. Functions, Recursion & Clean Scopes
fn fibonacci(n) {
    if n <= 1 { return n; }
    return fibonacci(n - 1) + fibonacci(n - 2);
}

println("Fibonacci(10): " + to_string(fibonacci(10)));`,
    logs: `[AETHER JIT] Compiled fibonacci recursion...
Fibonacci(10): 55
[Call depth: 10 frames - Success]`
  },
  fibers: {
    code: `// 04. M:N Green Fibers (1 Million Concurrent Tasks)
let ch = channel();

spawn(fn() {
    let sum = 0;
    for i in 0..50000 { sum = sum + i; }
    ch.send(sum);
});

println("Received from background fiber: " + to_string(ch.recv()));`,
    logs: `[Fiber Scheduler] Spawned green fiber on work-stealing pool...
Received from background fiber: 1249975000
[Concurrency throughput: 1.2M fibers/sec]`
  },
  quantum: {
    code: `// 05. Advanced: Post-Quantum Simulation
let reg = QuantumRegister(2);
reg.h(0);
reg.cnot(0, 1);
let outcome = reg.measure(0);
println("Measured entangled Q0: " + to_string(outcome));`,
    logs: `[Quantum Simulator] Initialized 2-qubit state vector |00⟩
[Quantum Simulator] Applied Hadamard Gate to Q0
[Quantum Simulator] Applied CNOT Gate (Control: Q0, Target: Q1)
Measured entangled Q0: 1
[Wavefunction collapsed via Born rule]`
  }
};

const editor = document.getElementById("code-editor");
const logs = document.getElementById("console-logs");

function switchTab(tab, event) {
  // Update tabs active state
  document.querySelectorAll(".tab-btn").forEach(btn => btn.classList.remove("active"));
  if (event) {
    event.currentTarget.classList.add("active");
  }

  // Load sample content
  editor.value = samples[tab].code;
  logs.innerText = samples[tab].logs;
}

function copyInstall(type) {
  let elementId = "install-cmd";
  if (type === "ps") elementId = "install-cmd-ps";
  else if (type === "bash") elementId = "install-cmd-bash";
  
  const el = document.getElementById(elementId) || document.getElementById("install-cmd");
  if (!el) return;
  
  const cmd = el.innerText;
  navigator.clipboard.writeText(cmd);
  
  // Custom button feedback
  const eventBtn = window.event ? window.event.currentTarget : document.querySelector(".copy-btn");
  if (eventBtn) {
    const originalText = eventBtn.innerText;
    eventBtn.innerText = "Copied! ✓";
    eventBtn.style.backgroundColor = "#22d3ee";
    eventBtn.style.color = "#000";
    setTimeout(() => {
      eventBtn.innerText = originalText;
      eventBtn.style.backgroundColor = "rgba(34, 211, 238, 0.1)";
      eventBtn.style.color = "var(--accent-color)";
    }, 2000);
  }
}

// Initial load
switchTab("hello");

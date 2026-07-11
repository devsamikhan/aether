const samples = {
  hello: {
    code: `// Hello World in AETHER
intent HelloAether {
    schema {
        message: String = "Hello, Open Source World!";
    }

    fn run() {
        println(this.message);
    }
}`,
    logs: `[Toolchain] Scaffolding AST nodes for HelloAether...
[JIT Lowering] Lowering Statement to CPU: MOV
Hello, Open Source World!`
  },
  quantum: {
    code: `// Quantum superposition in AETHER
intent QuantumRNG {
    fn generate() {
        qubit q;
        superpose(q);
        measure(q) => val;
        return val;
    }
}`,
    logs: `[Quantum Compiler] Qubit allocated at index 0
[Quantum Compiler] Applied Hadamard Gate to Q0
  -> Superposition state matrix loaded.
[Quantum Compiler] Collapsing wave-function for Q0...
  -> Random outcome eigenvalue collapse: 1`
  },
  multiverse: {
    code: `// Multiverse pathfinding speculation
intent specPath {
    fn query() {
        branch_reality {
            ManyWorldsPathfind(graph: grid, dest: target);
            observe_timeline(outcome);
        };
        merge_universe(outcome);
    }
}`,
    logs: `[Multiverse JIT] Forked UCG graph into target speculation timelines...
[Multiverse JIT] speculative traversal of 10,000 sub-routes completed.
[Multiverse JIT] Selected timeline outcome based on min-cost selection.`
  }
};

const editor = document.getElementById("code-editor");
const logs = document.getElementById("console-logs");

function switchTab(tab) {
  // Update tabs active state
  document.querySelectorAll(".tab-btn").forEach(btn => btn.classList.remove("active"));
  event.target.classList.add("active");

  // Load sample content
  editor.value = samples[tab].code;
  logs.innerText = samples[tab].logs;
}

function copyInstall() {
  const cmd = document.getElementById("install-cmd").innerText;
  navigator.clipboard.writeText(cmd);
  alert("Installation command copied to clipboard!");
}

// Initial load
switchTab("hello");

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

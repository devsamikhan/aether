# AETHER 2.0 User Guide: Quick Start

## 1. Verify Installation
Ensure that the `aether` binary is available in your PATH:

```bash
aether doctor
```

---

## 2. Create Your First Project
Use the built-in scaffolding tool:

```bash
aether new hello_aether
cd hello_aether
```

---

## 3. Write Code
Edit `src/main.ae`:

```aether
intent Application {
    schema {
        app_name: String = "AETHER Engine";
    }

    fn run() {
        println("Welcome to: " + this.app_name);
        let numbers = [1, 2, 3, 4, 5];
        let squared = [x * x for x in numbers];
        println("Calculated squares: " + to_string(squared));
    }
}

let app = Application();
app.run();
```

---

## 4. Run the Project
Execute with zero configuration:

```bash
aether run src/main.ae
```
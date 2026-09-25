# AETHER 2.0 Language Reference: Declarative Intent Contracts

## 1. Overview & Motivation
In standard programming languages, code expresses *imperative mutation* without communicating *declarative intent*. AETHER introduces the first-class `intent` contract primitive, enforcing formal invariant verification, preconditions, and postconditions directly at compilation and runtime.

---

## 2. Intent Syntax & Structure

An `intent` declaration consists of:
1. `schema`: Field definitions, static types, and default initializers.
2. `require`: Preconditions that must evaluate to `true` before method invocation.
3. `ensure`: Postconditions that must evaluate to `true` upon function completion.
4. `fn` blocks: Action methods that execute under the verified invariant contract.

```aether
intent BankAccount {
    schema {
        owner: String;
        balance: Float = 0.0;
        is_frozen: Bool = false;
    }

    require {
        this.balance >= 0.0;
        not this.is_frozen;
    }

    ensure {
        this.balance >= 0.0;
    }

    fn deposit(amount) {
        if amount <= 0.0 {
            throw "InvalidDepositAmount";
        }
        this.balance = this.balance + amount;
    }

    fn withdraw(amount) {
        if amount > this.balance {
            throw "InsufficientFunds";
        }
        this.balance = this.balance - amount;
    }
}
```

---

## 3. Instantiation & Verification
Instantiating an `intent` executes schema validation. Preconditions (`require`) are asserted before any method call, and postconditions (`ensure`) are verified upon method exit:

```aether
let acc = BankAccount("Sami", 1000.0);
acc.deposit(500.0);
println("New balance: " + to_string(acc.balance)); # 1500.0
```

If an invariant is breached at any point during mutation, the runtime raises an immediate `ContractViolationError` and rolls back state.
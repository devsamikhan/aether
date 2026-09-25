# Chapter 8: The Core Paradigm: Declarative Intent Contracts

The defining innovation of AETHER is the **Intent Contract**. An intent bounds execution behavior with formal invariants.

### 8.1 Anatomy of an Intent Contract

```aether
intent transfer_funds(sender_bal, receiver_bal, amount):
    require: sender_bal >= amount
    require: amount > 0
    ensure: result["sender"] == sender_bal - amount
    ensure: result["receiver"] == receiver_bal + amount
    body:
        let new_sender = sender_bal - amount
        let new_receiver = receiver_bal + amount
        return {
            "sender": new_sender,
            "receiver": new_receiver
        }

let settlement = transfer_funds(1000, 200, 350)
println("Settlement Result:", settlement)
# {"sender": 650, "receiver": 550}
```

If any `require` precondition is violated (e.g. `amount <= 0` or insufficient balance), the runtime halts execution before any state mutation can occur.

---

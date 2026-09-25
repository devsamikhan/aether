# Chapter 6: Object-Oriented Programming & Zero-Cost Structs

### 6.1 Classes, Constructors & Encapsulation

```aether
class BankAccount:
    def __init__(self, owner, initial_balance=0.0):
        self.owner = owner
        self.balance = initial_balance

    def deposit(self, amount):
        if amount <= 0:
            println("Error: Deposit amount must be positive.")
            return false
        self.balance = self.balance + amount
        return true

    def withdraw(self, amount):
        if amount > self.balance:
            println("Error: Insufficient funds.")
            return false
        self.balance = self.balance - amount
        return true

let account = BankAccount("Latif", 1000.0)
account.deposit(500.0)
account.withdraw(200.0)
println("Current Balance:", account.balance) # 1300.0
```

### 6.2 Class Inheritance & Polymorphism

```aether
class Animal:
    def __init__(self, name):
        self.name = name

    def speak(self):
        return "..."

class Dog(Animal):
    def speak(self):
        return "Woof! Woof!"

let pet = Dog("Rex")
println(pet.name + " says: " + pet.speak())
```

---

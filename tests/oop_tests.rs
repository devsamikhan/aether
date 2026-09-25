use aether::vm::{run_source, Value};

#[test]
fn test_class_definition_and_instantiation() {
    let code = r#"
class Person:
    def set_info(self, name, age):
        self.name = name
        self.age = age
    def info(self):
        return self.name + " is " + str(self.age)

p = Person()
p.set_info("Alice", 30)
[p.name, p.age, p.info()]
"#;
    let result = run_source(code).expect("Class definition and instantiation failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::string("Alice"));
        assert_eq!(items[1], Value::Int(30));
        assert_eq!(items[2], Value::string("Alice is 30"));
    } else {
        panic!("Expected array of results, got {:?}", result);
    }
}

#[test]
fn test_constructor_init() {
    let code = r#"
class Car:
    def __init__(self, brand, year=2024):
        self.brand = brand
        self.year = year

class Bike:
    def init(self, brand, gears=6):
        self.brand = brand
        self.gears = gears

c1 = Car("Tesla")
c2 = Car("Ford", 2020)
c3 = Car(brand="BMW", year=2022)
b1 = Bike("Yamaha", gears=5)
[c1.brand, c1.year, c2.brand, c2.year, c3.brand, c3.year, b1.brand, b1.gears]
"#;
    let result = run_source(code).expect("Constructors failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::string("Tesla"));
        assert_eq!(items[1], Value::Int(2024));
        assert_eq!(items[2], Value::string("Ford"));
        assert_eq!(items[3], Value::Int(2020));
        assert_eq!(items[4], Value::string("BMW"));
        assert_eq!(items[5], Value::Int(2022));
        assert_eq!(items[6], Value::string("Yamaha"));
        assert_eq!(items[7], Value::Int(5));
    } else {
        panic!("Expected array of results, got {:?}", result);
    }
}

#[test]
fn test_instance_methods_and_self() {
    let code = r#"
class Counter:
    def __init__(self, initial=0):
        self.count = initial
    def increment(self, step=1):
        self.count += step
        return self.count
    def reset(self):
        self.count = 0

c1 = Counter()
c2 = Counter(10)
c1.increment()
c1.increment(4)
c2.increment(5)
[c1.count, c2.count]
"#;
    let result = run_source(code).expect("Methods and self mutation failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(5));
        assert_eq!(items[1], Value::Int(15));
    } else {
        panic!("Expected array of results, got {:?}", result);
    }
}

#[test]
fn test_inheritance_and_method_overriding() {
    let code = r#"
class Animal:
    def __init__(self, name):
        self.name = name
    def speak(self):
        return "generic animal sound"
    def eat(self):
        return self.name + " is eating"

class Dog(Animal):
    def speak(self):
        return "Woof!"

class Cat(Animal):
    def speak(self):
        return "Meow!"

d = Dog("Buddy")
c = Cat("Whiskers")
[d.name, d.eat(), d.speak(), c.name, c.eat(), c.speak()]
"#;
    let result = run_source(code).expect("Inheritance and overriding failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::string("Buddy"));
        assert_eq!(items[1], Value::string("Buddy is eating"));
        assert_eq!(items[2], Value::string("Woof!"));
        assert_eq!(items[3], Value::string("Whiskers"));
        assert_eq!(items[4], Value::string("Whiskers is eating"));
        assert_eq!(items[5], Value::string("Meow!"));
    } else {
        panic!("Expected array of results, got {:?}", result);
    }
}

#[test]
fn test_super_delegation() {
    let code = r#"
class Employee:
    def __init__(self, name, salary):
        self.name = name
        self.salary = salary
    def get_role(self):
        return "Employee"

class Manager(Employee):
    def __init__(self, name, salary, bonus):
        super().__init__(name, salary)
        self.bonus = bonus
    def total_comp(self):
        return self.salary + self.bonus
    def get_role(self):
        return "Manager"

m = Manager("Alice", 100000, 25000)
[m.name, m.salary, m.bonus, m.total_comp(), m.get_role()]
"#;
    let result = run_source(code).expect("Super delegation failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::string("Alice"));
        assert_eq!(items[1], Value::Int(100000));
        assert_eq!(items[2], Value::Int(25000));
        assert_eq!(items[3], Value::Int(125000));
        assert_eq!(items[4], Value::string("Manager"));
    } else {
        panic!("Expected array of results, got {:?}", result);
    }
}

#[test]
fn test_encapsulation_and_getters_setters() {
    let code = r#"
class BankAccount:
    def __init__(self, owner, balance=0):
        self.owner = owner
        self._balance = balance
    def get_balance(self):
        return self._balance
    def deposit(self, amount):
        if amount > 0:
            self._balance += amount
            return True
        return False
    def withdraw(self, amount):
        if amount > 0 and amount <= self._balance:
            self._balance -= amount
            return True
        return False

acc = BankAccount("Bob", 500)
dep_ok = acc.deposit(250)
w1_ok = acc.withdraw(100)
w2_ok = acc.withdraw(1000)
[acc.get_balance(), dep_ok, w1_ok, w2_ok]
"#;
    let result = run_source(code).expect("Encapsulation failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(650));
        assert_eq!(items[1], Value::Bool(true));
        assert_eq!(items[2], Value::Bool(true));
        assert_eq!(items[3], Value::Bool(false));
    } else {
        panic!("Expected array of results, got {:?}", result);
    }
}

#[test]
fn test_polymorphism() {
    let code = r#"
class Shape:
    def area(self):
        return 0

class Rectangle(Shape):
    def __init__(self, width, height):
        self.width = width
        self.height = height
    def area(self):
        return self.width * self.height

class Square(Rectangle):
    def __init__(self, side):
        super().__init__(side, side)

shapes = [Rectangle(4, 5), Square(6)]
total_area = 0
for s in shapes:
    total_area += s.area()
total_area
"#;
    let result = run_source(code).expect("Polymorphism failed");
    assert_eq!(result, Value::Int(56)); // 20 + 36 = 56
}

#[test]
fn test_abstraction_pass_methods() {
    let code = r#"
class DatabaseConnector:
    def connect(self):
        pass
    def execute(self, query):
        pass

class MockDB(DatabaseConnector):
    def __init__(self):
        self.connected = False
    def connect(self):
        self.connected = True
        return "connected"
    def execute(self, query):
        if self.connected:
            return "executed: " + query
        return "error: not connected"

db = MockDB()
r1 = db.execute("SELECT 1")
c_res = db.connect()
r2 = db.execute("SELECT 1")
[r1, c_res, r2]
"#;
    let result = run_source(code).expect("Abstraction with pass failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::string("error: not connected"));
        assert_eq!(items[1], Value::string("connected"));
        assert_eq!(items[2], Value::string("executed: SELECT 1"));
    } else {
        panic!("Expected array of results, got {:?}", result);
    }
}

#[test]
fn test_class_variables() {
    let code = r#"
class Dog:
    species = "Canis familiaris"
    def __init__(self, name):
        self.name = name

d1 = Dog("Fido")
d2 = Dog("Rex")
c_spec = Dog.species
Dog.species = "Wolf"
[c_spec, Dog.species, d1.species, d2.species]
"#;
    let result = run_source(code).expect("Class variables failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::string("Canis familiaris"));
        assert_eq!(items[1], Value::string("Wolf"));
        assert_eq!(items[2], Value::string("Wolf"));
        assert_eq!(items[3], Value::string("Wolf"));
    } else {
        panic!("Expected array of results, got {:?}", result);
    }
}

#[test]
fn test_magic_methods_operator_overloading_and_str_len() {
    let code = r#"
class Vector2D:
    def __init__(self, x, y):
        self.x = x
        self.y = y
    def __add__(self, other):
        return Vector2D(self.x + other.x, self.y + other.y)
    def __sub__(self, other):
        return Vector2D(self.x - other.x, self.y - other.y)
    def __mul__(self, scalar):
        return Vector2D(self.x * scalar, self.y * scalar)
    def __eq__(self, other):
        return self.x == other.x and self.y == other.y
    def __str__(self):
        return "(" + str(self.x) + ", " + str(self.y) + ")"
    def __getitem__(self, idx):
        if idx == 0:
            return self.x
        elif idx == 1:
            return self.y
        return nil

v1 = Vector2D(3, 4)
v2 = Vector2D(1, 2)
v3 = v1 + v2
v4 = v1 - v2
v5 = v1 * 2
is_eq = (v3 == Vector2D(4, 6))
s_rep = str(v1)
[v3.x, v3.y, v4.x, v4.y, v5.x, v5.y, is_eq, s_rep, v1[0], v1[1]]
"#;
    let result = run_source(code).expect("Magic methods and operator overloading failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(4));
        assert_eq!(items[1], Value::Int(6));
        assert_eq!(items[2], Value::Int(2));
        assert_eq!(items[3], Value::Int(2));
        assert_eq!(items[4], Value::Int(6));
        assert_eq!(items[5], Value::Int(8));
        assert_eq!(items[6], Value::Bool(true));
        assert_eq!(items[7], Value::string("(3, 4)"));
        assert_eq!(items[8], Value::Int(3));
        assert_eq!(items[9], Value::Int(4));
    } else {
        panic!("Expected array of results, got {:?}", result);
    }
}

#[test]
fn test_magic_methods_getitem_setitem_container() {
    let code = r#"
class CustomList:
    def __init__(self):
        self.data = [0, 0, 0]
    def __getitem__(self, idx):
        return self.data[idx]
    def __setitem__(self, idx, val):
        self.data[idx] = val
    def __len__(self):
        return len(self.data)

c = CustomList()
c[0] = 10
c[1] = 20
c[2] = 30
[c[0], c[1], c[2], len(c)]
"#;
    let result = run_source(code).expect("Getitem/Setitem container failed");
    if let Value::Array(arr) = result {
        let items = arr.lock().clone();
        assert_eq!(items[0], Value::Int(10));
        assert_eq!(items[1], Value::Int(20));
        assert_eq!(items[2], Value::Int(30));
        assert_eq!(items[3], Value::Int(3));
    } else {
        panic!("Expected array of results, got {:?}", result);
    }
}

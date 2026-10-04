# HyperPulse Web Framework (Pure AETHER)

**HyperPulse** is a high-throughput, asynchronous HTTP & REST microservices framework written entirely in pure **AETHER**.

## Features

- **Expressive Routing**: Decorator-like route registration (`GET`, `POST`, `PUT`, `DELETE`).
- **Dynamic URL Parameters**: Seamless parameter extraction like `/api/v1/users/:id`.
- **Pipeline Middleware**: Composable middleware chains utilizing AETHER's `|>` pipeline operator.
- **Pythonic Ergonomics**: Native string interpolation via Python-style F-Strings (`f"{expr}"`).
- **High-Performance**: Evaluates over **7,000+ requests/sec** in simulated microservice workloads.

## Quick Start

```python
app = HyperPulse("MyService")

# Add Middleware
app.use(log_middleware)

# Define Handlers
def get_user(req):
    user_id = req.params.get("id")
    return Response(200, f'{{"user_id": "{user_id}", "status": "active"}}', "application/json")

app.route("GET", "/users/:id", get_user)

# Execute via pipeline
response = req |> app.dispatch
```

## Running the Project

```bash
aether run flagship_projects/01_hyperpulse_web/main.ae
# Or compile into standalone native Windows executable:
aether build flagship_projects/01_hyperpulse_web/main.ae -o hyperpulse.exe
```

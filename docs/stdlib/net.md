# Standard Library: `std::net` & `std::http`

## Overview
Socket abstractions, TCP listener primitives, and HTTP request handlers.

---

## Functions

### `http_get(url: String) -> String`
Performs an HTTP GET request and returns the response body string.

### `http_post(url: String, body: String, headers: Map) -> String`
Performs an HTTP POST request with given payload and headers.

### `tcp_listen(address: String, handler: Function) -> Nil`
Spawns non-blocking TCP listener with green-fiber connection dispatching.
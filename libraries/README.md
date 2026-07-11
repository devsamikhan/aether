# AETHER Libraries Directory

Welcome to the AETHER library directory.

## Directory Structure
- `std/`: Contains core standard library modules (`collections.aether`, `io.aether`, `net.aether`).
- `community/`: Third-party packages fetched by the package manager.

## Library Versioning
All libraries are stored in versioned subdirectories matching `~/.aether/libraries/<name>/<version>/`.

## Authoring new libraries
To write a library:
1. Declare an `intent` containing your functions and schema.
2. Publish by adding to a GitHub release tag inside the `libraries/` directory.
3. Users can import via `aether install <name>@<version>`.

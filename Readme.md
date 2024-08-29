# Synnq Validator

Welcome to the Synnq Validator, a core component of the Synnq network. This project is responsible for validating data, broadcasting transactions, and interacting with the discovery service to maintain a list of active nodes.

## Features

- **Node Registration**: Automatically registers the node with the discovery service.
- **Data Validation**: Uses Zero-Knowledge Proofs (ZKP) for data validation.
- **Transaction Broadcasting**: Efficiently broadcasts transactions across the network.
- **Node Discovery**: Continuously updates the node list from the discovery service.
- **Address Resolution**: Automatically resolves node addresses before starting the server.

## Repository Structure

- **synnq_val/**: The main folder containing the Synnq Validator source code.
- **config.json**: Configuration file where the node address is specified.

## Prerequisites

- **Rust**: Ensure you have the latest stable version of Rust installed.
- **Cargo**: Rust's package manager, which comes with Rust.
- **Actix Web**: The framework used for building the HTTP server.

## Installation

1. **Clone the Repository**:

   ```bash
   git clone https://github.com/synnq/synnq_val.git
   cd synnq_val
   ```

2. **Install Dependencies**:

   The project dependencies will be installed automatically when you build the project using Cargo.

   ```bash
   cargo build
   ```

## Configuration

The `config.json` file is used to specify the node's address. Ensure that this file is in the `synnq_val` folder before starting the node.

Example `config.json`:

```json
{
  "address": "http://node.synnq.io"
}
```

- **address**: The URL or IP address with a port where the node will be running. If a URL is provided, the node will resolve it before starting.

### Address Resolution

When the `address` in `config.json` is provided as a URL:

- The Synnq Validator will attempt to resolve the address by making a request to it before proceeding with node registration and server startup.
- If the address is a valid IP with a port, the address resolution step will be skipped.

This resolution process ensures that the node can connect to the provided address and avoid runtime errors due to incorrect or inaccessible URLs.

## Running the Node

To start the node, run:

```bash
cargo run --release
```

The node will:

1. Attempt to resolve the provided address (if it is a URL).
2. Register with the discovery service.
3. Fetch the list of active nodes.
4. Start the HTTP server and listen for incoming requests.

## Logging

Logging is managed using the `tracing` crate. Logs provide detailed information on the application's operations, including successful tasks and errors.

## Troubleshooting

- **Failed to Resolve Node Address**: Ensure that the provided node address in `config.json` is correct and accessible from your network.
- **Connection Refused**: Verify that the discovery service is running and reachable.

## Contributing

Contributions are welcome! Please open an issue or submit a pull request on [GitHub](https://github.com/synnq/synnq_val) with any improvements or bug fixes.

## License

This project is licensed under the MIT License. See the `LICENSE` file in the repository for more details.

## Contact

For any inquiries or support, please open an issue on [GitHub](https://github.com/synnq/synnq_val) or contact the maintainers through the repository.

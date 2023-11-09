# Proxy NDP (Neighbor Discovery Protocol)

## Overview

This is a Rust-based application designed to operate as a proxy for IPv6 Neighbor Discovery Protocol (NDP). The application listens for Neighbor Solicitation messages within a specified IPv6 prefix and responds with Neighbor Advertisement messages, effectively providing an on-link presence for the IPv6 addresses within the prefix. This is particularly useful in scenarios where direct IPv6 connectivity is not possible due to network constraints.

## Prerequisites
Before building and running the proxy-ndp application, make sure that you have:
- Docker installed and running on your system.
- Basic knowledge of Docker containerization concepts.

## Configuration
Currently, all configuration is done through command-line arguments as specified at runtime.

## Build Instructions

First, ensure you have Docker installed on your system. Then, follow these steps to build the proxy-ndp container image:

1. Clone the repository to your local machine and navigate to the repository directory.
2. Build the Docker image using the provided `Dockerfile`:

```shell
docker build -t proxy-ndp .
```

## Running the Application

To run the proxy-ndp application, execute it within a Docker container using the following command:

```shell
docker run --rm --network host proxy-ndp <NETWORK INTERFACE> <TARGET MAC ADDRESS> <PREFIX> <PREFIX LENGTH>
```

Make sure to replace `<NETWORK INTERFACE>`, `<TARGET MAC ADDRESS>`, `<PREFIX>`, and `<PREFIX LENGTH>` with appropriate values.

-    `<NETWORK INTERFACE>`: The name of the network interface to listen to.
-    `<TARGET MAC ADDRESS>`: The MAC address that will be used as the source in the Neighbor Advertisement.
-    `<PREFIX>`: The IPv6 prefix within which the application operates.
-    `<PREFIX LENGTH>`: The length of the IPv6 prefix (typically a number is 64).

For example:

```shell
docker run --rm --network host proxy-ndp eth0 00:11:22:33:44:55 fd00:: 64
```

## Usage
Once the application is running, it will passively listen for NDP Neighbor Solicitation messages and automatically respond as configured.

## Troubleshooting
If you experience any issues with running the proxy-ndp application, ensure all command-line arguments are correct and the Docker daemon has the necessary permissions to bind to network interfaces.

## License
This project is licensed under the Apache License, Version 2.0. See [LICENSE](http://www.apache.org/licenses/LICENSE-2.0) for the full license text.

## Credits
Special thanks to the following projects and libraries:
- [pnet](https://github.com/libpnet/libpnet): Rust library for packet manipulation.

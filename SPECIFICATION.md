# Reticulum DNS Protocol Specification

### **Abstract**
This document defines a **decentralized, encrypted DNS protocol** for the Reticulum network. It replaces traditional DNS with a **signed, queryable system** for resolving node destinations (e.g., `rns://weather.node`) while ensuring **privacy, integrity, and scalability**.

### **Status**
**Version:** 0.1

**Status:** Draft (Experimental)


### **Scope**
- **Not covered**:
  - Higher-level application protocols (e.g., Reticulum messaging).
- **Out of scope**:


### **Key Goals**
- [x] **Security**: End-to-end encrypted queries/answers, optional signing of messages.
- [x] **Semi decentralised**: DNS provider may choose one or more Authorites to fetch dns records from.
- [x] **Flexible**: Customization should be relatively easy without trying to hack and patch the underlying protocol.
- [x] **Simple**: The End user shouldn't have to worry too much about the underlying mechanisms.

## **Core Concepts**

### **Terminology**
| Term                   | Definition                                                                  |
|------------------------|-----------------------------------------------------------------------------|
| **Node**               | A participant in the Reticulum network (e.g., `rns://weather.node`).        |
| **Destination**        | An endpoint inside of the application space / virtual network.              |
| **Application Space**  | A virtual network which isolates the applications to prevent clutter.       |
| **Query**              | A DNS request                                                               |
| **Answer**             | A signed response containing node metadata (Destination, public key, TTL).  |
| **Routing Node**       | A node that relays queries.                                                 |
| **TTL (Time-to-Live)** | How long an answer is considered valid (e.g., 1 hour).                      |
| **Signature**          | A cryptographic proof that an answer is authentic (using Ed25519).          |

### **RNS URL Scheme**
- Format: `rns://TYPE/VERSION/...`
  - Example: `rns://D/VERSION/weather/InternetOfThings/7c9fa136d4413fa6173637e883b6998d//`
- **Types**:
  - `(N)`: Generic node information.
  - `(D)`: Destination node.
  - `(R)`: Routing node (future).

## **Protocol Design**

### **Packet Structure (Query)**
All messages are **binary-encoded** (not text-based like traditional DNS).

| Field          | Type       | Size (bytes) | Description                                     |
|----------------|------------|--------------|-------------------------------------------------|
| `id`           | `u16`      | 2            | Client-chosen request ID (for matching answers).|
| `answers`      | `u4`       | .5           | Number of answers per question.                 |
| `authority`    | `u4`       | .5           | Number of authorities per question.             |
| `level`        | `(u8, u8)` | 2            | Define which authorities should be included.    |
| `flags`        | `u8`       | 1            | Additional Flags.                               |
| `questions`    | `Vec<u8>`  | variable     | Domains (seperated by a limiter)                |

### **Example Query Packet**
TODO

## **Security**

### **Threat Model**
| Threat               | Mitigation                                             |
|----------------------|--------------------------------------------------------|
| **Spoofing**         | Ed25519 signatures ensure answers are authentic.       |
| **Eavesdropping**    | Queries/answers are encrypted (TLS or Reticulum E2EE). |
| **Denial-of-Service**| Rate limiting at routing nodes.                        |
| **Cache Poisoning**  | ?                                                      |

### **Cryptography**
- **Signatures**: Ed25519 (RFC 8032) for compact, fast signatures.
- **Encryption**: Optional TLS wrapper for transport security.
- **Key Management**: Nodes generate Ed25519 key pairs on startup.

### **Privacy**
- **No Logging**: Routing nodes should not log queries (like Tor).
- **Anonymity**: Use Reticulum’s E2EE for query payloads.

## **Implementation Details**

TODO

## **Open Questions**
- How to handle **reverse DNS** (e.g., resolving a destination back to a name)?
- Should it support **wildcard records** (e.g., `*.node`)?
- How to **scale routing nodes** in large networks?
- How to deal with Cache Poisoning?

## **Future Work**
- [ ] Add **DNSSEC-like validation** for trust chains.
- [ ] Implement **recursive resolution** (A DNS server should be able to sync from another server).
- [ ] Benchmark performance vs. traditional DNS (it will be much slower but it would be nice to see).

## Appendix
### References
[Reticulum Network](https://reticulum.network/)

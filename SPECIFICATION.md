# Reticulum DNS Protocol Specification
**Version:** 0.1
**Last Updated:** 2024-XX-XX
**Status:** Draft (Experimental)

### **Abstract**
This document defines a **decentralized, encrypted DNS protocol** for the Reticulum network. It replaces traditional DNS with a **signed, queryable system** for resolving node destinations (e.g., `rns://weather.node`) while ensuring **privacy, integrity, and scalability**.

### **Scope**
- **Not covered**:
  - Physical network layers (e.g., TCP/UDP).
  - Higher-level application protocols (e.g., Reticulum messaging).
- **Out of scope**:
  - Caching strategies (left to implementers).
  - Reverse DNS lookups (future work).

### **Key Goals**
- [x] **Security**: End-to-end encrypted queries/answers.
- [x] **Decentralization**: No single authority (unlike traditional DNS).
- [x] **Scalability**: Efficient resolution for large networks.
- [x] **Interoperability**: Works with Reticulum’s RNS URL scheme.

## **Core Concepts**

### **Terminology**
| Term                   | Definition                                                                  |
|------------------------|-----------------------------------------------------------------------------|
| **Node**               | A participant in the Reticulum network (e.g., `rns://weather.node`).        |
| **Destination**        | A cryptographic proof that an answer is authentic (using Ed25519).          |
| **Query**              | A DNS request (e.g., resolving `weather.node` to an Destination/key pair).  |
| **Answer**             | A signed response containing node metadata (Destination, public key, TTL).  |
| **Routing Node**       | A node that relays queries (acts like a decentralized nameserver).          |
| **TTL (Time-to-Live)** | How long an answer is considered valid (e.g., 1 hour).                      |
| **Signature**          | A cryptographic proof that an answer is authentic (using Ed25519).          |

### **RNS URL Scheme**
- Format: `rns://TYPE/VERSION/...`
  - Example: `rns://D/VERSION/weather/InternetOfThings/7c9fa136d4413fa6173637e883b6998d`
- **Types**:
  - `(D)`: Destination node.
  - `(R)`: Routing node (future).

## **Protocol Design**

### **Packet Structure**
All messages are **binary-encoded** (not text-based like traditional DNS).

| Field          | Type       | Size (bytes) | Description                                  |
|----------------|------------|--------------|----------------------------------------------|
| `magic`        | `u32`      | 4            | `0x52455449` (ASCII "RETI") for identification. |
| `version`      | `u8`       | 1            | Protocol version (e.g., `1`).                |
| `packet_type`  | `u8`       | 1            | `0` = Query, `1` = Answer.                   |
| `id`           | `u16`      | 2            | Client-chosen request ID (for matching answers). |
| `payload`      | `str`      | Variable     | Query: `rns://weather.node`. Answer: Signed node data. |
| `signature`    | `vec<u8>`  | Variable     | Ed25519 signature over `payload`.            |

### **Example Query Packet**
TODO

## **Security**

### **Threat Model**
| Threat               | Mitigation                                  |
|----------------------|---------------------------------------------|
| **Spoofing**         | Ed25519 signatures ensure answers are authentic. |
| **Eavesdropping**    | Queries/answers are encrypted (TLS or Reticulum E2EE). |
| **Denial-of-Service**| Rate limiting at routing nodes.            |
| **Cache Poisoning**  | Short TTLs (e.g., 1 hour) and signature validation. |

### **Cryptography**
- **Signatures**: Ed25519 (RFC 8032) for compact, fast signatures.
- **Encryption**: Optional TLS wrapper for transport security.
- **Key Management**: Nodes generate Ed25519 key pairs on startup.

### **Privacy**
- **No Logging**: Routing nodes should not log queries (like Tor).
- **Anonymity**: Use Reticulum’s E2EE for query payloads if needed.

---
## **Implementation Details**

## **Open Questions**
- How to handle **reverse DNS** (e.g., resolving an IP back to a name)?
- Should we support **wildcard records** (e.g., `*.node`)?
- How to **scale routing nodes** in large networks?

## **Future Work**
- [ ] Add **DNSSEC-like validation** for trust chains.
- [ ] Implement **recursive resolution** (like traditional DNS).
- [ ] Benchmark performance vs. traditional DNS.

## Appendix
### References

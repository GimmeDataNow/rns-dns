# Specification

This file will serve as the specification for the project.

## Dns server


message_to_sign = hash("weather.node" + destination + timestamp)
signature = sign_with_private_key(message_to_sign)

verify_signature(
    public_key=destination_public_key,
    signature=provided_signature,
    message_hash=hash("weather.node" + destination + timestamp)
)

This dns system is made for systems with extremely long ttl entries since the destinations are unique.

respond with error codes

0x00	✅ Success (name found)
0x01	❌ NXDOMAIN (name not found)
0x02	🔒 Not authorized (unauthorized update)
0x03	🕒 Expired (record known, but expired)
0x04	🧾 Invalid signature
0x05	🚫 Verification required (name exists but not trusted)
0x06	🧍 Identity mismatch (wrong owner tried to update) // unlikely error. might reduce the index
0x07	🧱 System error or overload

### entering into a record
1. check if the name/suffix is available and is not reserved
2. provide the public key form which the destination is computed


### verifying a record
1. enter the dns record
2. request a verification from an authority
3. obtain the verification
4. maintain and update the verification

### obtaining a record

TODO

## Connection

Specifies how the server, client, and routers connect and communicate with each other.

### connecting to the destination server

- A routing node is required.
- This routing node does not need to be connected to any virtual network/aplication namespace.
- The server needs to announce its presence to the routing node.
- The client does not need to announce its presence.
- The routing node might need to host a web server that broadcasts a cofig for the local network.
- this config would indicate what destinations there are on the local network and what destination to connect to for dns requests
- the routing node and the routing destination would be provided via qr-code (qr2term) or similar
- additional services on any device would require either env vars or a local file with the necessary dns info

## qr-code
- The qr-code for node adresses will be as follows

rns://TYPE-(N)/VERSION/PUBLIC_KEY/(ADDRESS_HASH, INTERFACE) (ADDRESS_HASH2, INTERFACE2)//

>! potential issues may be due to the way the interfaces store information (":" as a seperator).
This might cause issues for easy parsing for rns:// or for storing more than one rns:// url in one string.


rns://TYPE-(D)/VERSION/DESTINATION_NAME/APPLICATION_SPACE/ADDRESS_HASH,ADDRESS_HASH2//
>! this will require sanitization of the destination name and application space because otherwise someone
might inject malicious inputs

>it might be possible to reduce the number of hashes provided if there is some kind of reverse-look-up functionality for the public key.
Although this is probably a bad idea 


  1. node (N) or destination (D)
for N:
  VERSION: 1
  PUBLIC_KEY: WrgqoHGP4OjB3iAUylURkWQzyLqJuQ52GDEDD4ofa3w
  ADDRESS_HASH: 7c9fa136d4413fa6173637e883b6998d
  INTERFACE: Udp;0.0.0.0:4243:127.0.0.1:4242
for D:
  VERSION: 1
  DESTINATION_NAME: test-server
  APPLICATION_SPACE: app.1
  ADDRESSHASH: 7c9fa136d4413fa6173637e883b6998d


improvements:

Fix the way public keys currently work

eTDL with "use publicsuffix::List;"

since the entries are long lived it might be useful to track:
previous values in a seperate (slower) database
who made the last update
a reason for updating (probably in a seperate database)

add support for a single domain name pointing to multiple destinations

# Dns server


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

# entering into a record
1. check if the name/suffix is available and is not reserved
2. provide the public key form which the destination is computed


# verifying a record
1. enter the dns record
2. request a verification from an authority
3. obtain the verification
4. maintain and update the verification

# obtaining a record and connecting to the destination server

- A routing node is required.
- This routing node does not need to be connected to any virtual network/aplication namespace.
- The server needs to announce its presence to the routing node.
- The client does not need to announce its presence.
- The routing node might need to host a web server that broadcasts a cofig for the local network.
- this config would indicate what destinations there are on the local network and what destination to connect to for dns requests
- the routing node and the routing destination would be provided via qr-code (qr2term) or similar
- additional services on any device would require either env vars or a local file with the necessary dns info

improvements:

Fix the way public keys currently work

eTDL with "use publicsuffix::List;"

since the entries are long lived it might be useful to track:
previous values in a seperate (slower) database
who made the last update
a reason for updating (probably in a seperate database)

add support for a single domain name pointing to multiple destinations

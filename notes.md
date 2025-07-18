# Dns server

{
  "name": "weather.node",
  // destination node
  //"owner": "A1B2C3D4...", // not really needed
  "destination": "E5F6G7H8...", // this is at the same time the public key for signing
  "ttl": 3600,
  "timestamp": 1720673175,
  "signature": "<owner_signature>", // this is the name which is then encrypted using the private key which results in the signature
  "verifications": [ // this might have to be a ordered list where the server simply returns the 3 highest verifiers
    {
      "verifier_id": "1234ABCD...", // hash of the public key for ease of use and quick look ups
      "verifier_name": "rns-authority-1", // human readable name
      "verifier_public_key_or_destination": "<base64-encoded pubkey>", // public key (full)
      "verifier_signature": "<verifier_signature>" // weather node name encrypted using the private key of the verifier
    }
  ]
}

maybe only keep the verifier id and the signature and perform a reverse lookup if necessary
to keep records small and since verifiers might not appear that often
this could also contain a trust level / authority level


# Formal Dns Response

#### name
> the name

#### destination
> Both the destination and the public key at the same time which is used for the signature

#### ttl
> When the certificate expires

#### timestamp 
> When was this record last updated

#### signature
> The name of the node which is then encrypted using the private key of the destination node

#### verifications
> contains of the verifier id and the verifier signature

#### verifier_id
> it is the hash of the public key for ease of use. This is then used to perform a reverse look-up

#### verifier_signature
> this is the node name which is then  encrypted using the private key of the verifier

#### verifier_name
> human readable name of the authority

#### verifier_public_key_or_destination
> This is the public key of the authority and potentially also a destination

respond with error codes

0x00	✅ Success (name found)
0x01	❌ NXDOMAIN (name not found)
0x02	🔒 Not authorized (unauthorized update)
0x03	🕒 Expired (record known, but expired)
0x04	🧾 Invalid signature
0x05	🚫 Verification required (name exists but not trusted)
0x06	🧍 Identity mismatch (wrong owner tried to update)
0x07	🧱 System error or overload

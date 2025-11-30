import rns
import os
import time

# Use shared rnsd instance
rns.Transport.mode = rns.Transport.CLIENT
rns.Transport.start()

# Load or create identity
identity = rns.Identity.from_file("dns_client_identity") if os.path.exists("dns_client_identity") else rns.Identity()
if not os.path.exists("dns_client_identity"):
    identity.to_file("dns_client_identity")

# Replace with server destination hash
SERVER_HASH = bytes.fromhex("...")  # <- insert hash printed by server

# Link to the server
destination = rns.Destination(
    SERVER_HASH,
    rns.Destination.SINGLE,
    rns.Destination.PLAIN,
    "dns"
)

link = rns.Link(destination)

# Wait for link to become active
while not link.status == rns.Link.ACTIVE:
    rns.pulse()
    time.sleep(0.1)

# Send the query
query = "GET weather.node"
link.send(query.encode("utf-8"))

# Receive reply
def callback(link, data):
    print("[REPLY]", data.decode("utf-8"))
    link.teardown()

link.set_link_callback(callback)

# Keep script running to receive response
try:
    while link.status != rns.Link.CLOSED:
        rns.pulse()
        time.sleep(0.1)
except KeyboardInterrupt:
    link.teardown()

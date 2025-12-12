# TOFIX

---
## backend

parser:
return the '/' for the addresshashes

client:
current_link.lock().await.as_mut() is causing the process to stall if one of the links fails.
It will try to build links on all interfaces. This will make the programm stall.
> [!TIP]
> Only enable one path for now


path redundancy:
2 paths -> each send announce/ping -> server gets two pings -> responds with to the two pings on each interface -> 4 responses
this is exponential increase. This WILL CRASH
> [!TIP]
> Only enable one path for now
---
## tui

PgUp/PgDown bindings and scroll behaviour
implement a menu for passing on options through the visual mode

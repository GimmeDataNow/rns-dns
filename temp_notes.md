# Potential Options
- Private Identity from rand,string, or hex string
- Transport config (name, &private ident, true)
- Destination (app name, aspect)

according to claude the private identity between services should be seperate as it might cause linking issues.

        transport.send_packet(announcement).await;
hangs indefinitely after the first sent


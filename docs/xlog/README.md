# OM0RX-xNexus beside OM0RX-xLog

OM0RX-xNexus is full Nexus. It owns its own radio connection, decoding,
transmitting and logbook, exactly as upstream Nexus does. OM0RX-xLog remains the
station's master logbook. The two stay in step through two ordinary interfaces,
with no shared database and no special pairing:

| Direction | What | How |
|---|---|---|
| Nexus → xLog | Every contact logged in Nexus | Nexus **Settings ▸ Services ▸ WSJT-X UDP API** on, address `127.0.0.1:2237`. xLog's WSJT-X UDP listener saves each "logged ADIF" message. |
| xLog → Nexus | New contacts logged in xLog | xLog's sync agent (`tools/nexus_sync` in the xLog repository) sends each new contact as one raw ADIF record to Nexus's contact port, UDP `127.0.0.1:2239` ("Accept contacts other programs log", on by default). Contacts logged while Nexus is closed wait in the agent's queue. |
| xLog → Nexus | Confirmation changes (LoTW, QSL, eQSL) | The contact port refuses confirmation fields by design, so the agent writes them to `Documents\xLog-confirmations-for-Nexus.adi`; import that file in Nexus (Logbook ▸ Import ADIF). |

Nexus never forwards a contact-port contact to the local loggers, and its
duplicate check drops a contact it already holds, so nothing loops.

## What the fork adds to upstream Nexus

- **Identity.** Product name `OM0RX-xNexus`, bundle identifier `sk.om0rx.xnexus`,
  data under `%APPDATA%\om0rx-xnexus` and `OM0RX-xNexus` folders, keychain
  service names, and versions `1.25.0-xlog.N`, so it installs beside upstream
  Nexus without sharing settings or logbook. The upstream self-updater is
  disabled: the fork has no signed release channel of its own.
- **CS and DS columns** in the Call Roster and Band Activity: call and DXCC status
  in your own Nexus logbook on the current band, by mode class (CW, phone,
  digital), in xLog's codes and colours.
- **Native-Windows build and installer tooling** under `scripts/xnexus/`
  (GNU toolchain, guarded memory-bounded packaging, hardened NSIS template; see
  [WINDOWS_DISTRIBUTION_HARDENING.md](WINDOWS_DISTRIBUTION_HARDENING.md)).
- `UPSTREAM.json` records the upstream commit last merged.

The earlier xLog companion mode (`--xlog-companion`, `integrations/xlog*`,
`ui/src/xlog`) was removed in 1.25.0-xlog.17; it remains in Git history.

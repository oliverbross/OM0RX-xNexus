# Using OM0RX-xNexus with xLog

This guide describes the integrated source. Check [STATUS.md](STATUS.md) and the
candidate's build receipt before treating a build as qualified. Both programs
run separately on the same computer. The Mac companion requires macOS14 or newer
for an isolated persistent WebView store. A bridge-enabled xLog build is required;
an older xLog installation will not acquire the bridge merely by installing
xNexus.

## Pair the applications

1. Open your intended workspace in xLog and leave xLog running. Configure the
   radio, rotor and cluster in xLog as usual.
2. In xLog's service settings, find **OM0RX-xNexus companion** and choose
   **Enable and create pairing file**. This grants local companion access to
   the advertised observations, history, controls and QSO writeback.
3. Open OM0RX-xNexus and choose **Open xLog pairing file**. Select the exact
   private file whose path xLog displays.
4. Check the connection status and workspace before operating. The application
   shows xLog's station observations and needed spots when they are available.

The token remains in the native application and is forgotten when xNexus exits.
The pairing file is a credential, not a settings file to share with other people.
Pairing is disabled on each xLog launch. After an xLog restart or workspace
change, enable its bridge and open the new pairing file. The durable QSO queue
remains on this computer and stays bound to its original workspace.

## Choose a station

Use **Needed** for xLog's current priorities or **All spots** for the full bounded
cluster snapshot. Filter by callsign and choose a station from the list, Nexus
band map or world map. Need status, score and reasons come from xLog. Unknown
status is not treated as unworked. World markers use only supplied coordinates;
an unpositioned observation remains available in the list.

**Load history from xLog** explicitly reads up to 25 recent contacts and the
owner's band/mode worked/confirmed matrix. Refresh it after a log change. A
history row without an exact edit revision is view-only.

## Control the station

**Tune through xLog** requests xLog's simplex frequency/mode operation on the
observed radio. Split configuration and device selection remain in xLog.
Transmit or unknown receive state disables the tune action. **Point through
xLog** sends the entered azimuth to xLog's rotor owner; xLog applies travel limits.
Selecting a map marker alone never tunes or moves anything.

**Stop rotator** has an independent route and stays visible. After an ordinary
disconnect, it may target the explicitly labelled last paired rotor. Forgetting
the pairing or starting a different pairing clears that route. Revoking access
in xLog stops further admission but does not undo movement already requested.

An accepted receipt is an acknowledgement. An observed receipt means xLog
reports the target observation. An uncertain result must be checked in xLog
before another ordinary control action. The companion never automatically
repeats an uncertain tune or movement.

## Log and correct a QSO

Fill the QSO draft, or use the selected station. Check its UTC start time,
frequency, reports, grid and comment, then choose **Queue QSO**. This persists
the draft locally; it does not yet save it in xLog. Choose **Submit to xLog**
on the retained entry to send it. Only an applied/observed receipt naming the
contact and its revision establishes the owner write.

The unfinished draft is saved in this companion profile and restored after an
ordinary close. Its operation ID, original workspace and edit revision are
retained. Reconnecting to a different log does not redirect it there. **Clear
draft** deliberately starts a new operation; loading an edit from history also
requires an empty draft. A storage error keeps the current values visible and
blocks submission until they can be saved.

If a reply is lost or either process stops, the entry says **Receipt required**.
Use **Check receipt**. **Retry only if receipt missing** first checks the owner
again and reuses the same immutable operation ID; it never invents another
contact to recover a lost reply. Keep the original entry until its outcome is
resolved. A queue persistence error after dispatch does not mean nothing was sent.

Use **Edit QSO** in loaded history to prepare a correction at that exact contact
revision. The owner rejects a conflict if another edit has changed the contact.
Callsign, band and mode changes require xLog's editor, which owns the retained
entity and confirmation evidence. The companion preserves other fields and
never performs a separate online-log upload; xLog owns upload policy.

The queue currently retains up to 10,000 operations, including terminal receipts.
It has no automatic deletion or archival UI. Do not delete queue files to recover
an uncertain write; resolve its owner receipt first.

## Receive FT8 or FT4 through xLog

In a bridge-enabled Windows xLog, select and explicitly confirm the FLEX slice A
and DAX input mapping in companion settings. The channel must match the radio's
observed DAX channel and the active Windows endpoint; do not assume RX 1. Select
the native input format/channel and start RX sharing in Normal Logging.

Pair xNexus, choose FT8 or FT4 under **Receive decoder**, then **Start receiving**.
The decoder mode is separate from the radio's DIGU setting. Wait for a complete
uninterrupted period. A gap, changed route, expired connection or decoder backlog
discards the partial period. **Stop receiving** stops the companion consumer;
xLog retains control of capture/sharing. Stop before changing decoder mode.

The receive panel retains up to 200 decoded rows. They are observations and do
not automatically tune, classify need, transmit or create a QSO. Need badges on
the station board still come from xLog's classified cluster spots. Timing uses
the host audio endpoint clock; global UTC accuracy and radio-to-endpoint delay
have not been measured. See [RX_DESKTOP.md](RX_DESKTOP.md) for qualification.

## Current boundaries

Companion mode has no automatic QSO sequence or transmit control. Physical DAX
receive qualification is still pending. Closing xLog leaves xNexus open with
unavailable station actions and its retained queue; it never falls back to a
second hardware or cluster connection.

The optional developer `--independent-mode` launch uses the retained upstream
application path. It is a separate operating mode, not connection recovery, and
its hardware operation has not been qualified by companion tests.

For disposable native acceptance, launch the exact candidate executable with
`--xlog-data-dir <absolute disposable root>`. That root holds its companion outbox. Windows stores its WebView profile there;
on macOS the canonical root selects a distinct OS-managed persistent WebView store. Verify the live process path and complete arguments
before any UI observation. A failed launch is a stop, never permission to open
the application without the isolation argument.

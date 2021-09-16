# VRPN Protocol Details

Based on a post-07.34 revision of mainline (C++) VRPN,
both analysis of code as well as study of network traces.

General notes:

- Values are marshalled as big-endian, aka "network byte order".
- There is no single consistent convention for marshalling strings - it varies per message type.
- Many messages, once the initial connection handshake takes place, have a common header/wrapper form.
- A single datagram/packet may contain multiple messages.
- Many, though not all, fields are padded to a multiple of `vrpn_ALIGN` (8) bytes.
  Whether this padding is included in field-length values varies.
- Sender IDs and message type IDs are dynamically allocated.
  They can be different on each side of the connection,
  or between different connections.
  They map to corresponding unique and stable string identifiers,
  which are communicated via `SENDER_DESCRIPTION` and `TYPE_DESCRIPTION` messages, respectively.
- Quaternions are w, x, y, z

## Common message framing

Normally-framed messages (after initial handshaking) consists of two logically-separate components:

- The header, which has the same form for all messages post-handshake.
- The body, which varies by message type,
  and which is logically handled by the main marshalling code in mainline
  generically as a byte array with a length.

The header consists of five (5) 32-bit integers which form four (4) fields:

- length
- timestamp (as `struct timeval`: seconds and microseconds parts, two (2) 32-bit integers)
- sender ID
- message type ID

The value of the `length` field is the sum of padded length of the header and
the *unpadded* length of the body.
The header length, unpadded, is always `5 * 4 = 20` bytes, which results in a
padded length of `24` bytes.

In the "padding" following the header, there is actually a sixth `u32`:
a sequence number.
The mainline does not deserialize it, and it is not included in the length field.

The header is then padded out to a multiple eight (8) bytes before starting the body.
This overlaps completely with the (optional) sequence number field,
so if you include the sequence number in the header, there is no padding for the header.
The body is padded out (typically with 0) to a multiple of `vrpn_ALIGN` (8).

Additionally, messages may have a "class of service" specified.
The main usage for this is distinguishing "reliable" (send via TCP) and
"low-latency" (send via UDP) when a UDP+TCP connection is available.

## Common message payloads

The payload format of the message body is defined by the message type identifier,
which maps to the dynamically allocated message type ID.
The following is an incomplete list of a few common message types,
in alphabetical order of the message type identifier string:

### "vrpn_Analog Channel"

This message contains readings for a number of analog channels.
The message body consists of a number of 64-bit floating point values.
The first value contains the number of channels that follow.
The remaining values represent one analog channel each:

- `num_channels` (`f64`)
- `channel[0]` (`f64`)
- ...
- `channel[num_channels - 1]` (`f64`)

The number and actual meaning of these values depends on the sender.

### "vrpn_Button Change"

This message contains new button states for a subset of the buttons on the sender.
The message body consists of a number of 32-bit signed integer values.
The first value contains the number of button changes that follow.
The remaining values are pairs of button id and button state:

- `num_buttons` (`i32`)
- `button_id[0]` (`i32`)
- `button_state[button_id[0]]` (`i32`)
- ...
- `button_id[num_buttons - 1]` (`i32`)
- `button_state[button_id[num_buttons - 1]]` (`i32`)

The `button_id`s correspond to the positions in the "vrpn_Button States"
message.

### "vrpn_Button States"

This message contains current button states for all buttons on the sender.
The message body consists of a number of 32-bit signed integer values.
The first value contains the number of button states that follow.
The remaining values each represent the state of a button on the sender:

- `num_buttons` (`i32`)
- `button_state[0]` (`i32`)
- ...
- `button_state[num_buttons - 1]` (`i32`)

The number of buttons and their order depends on the sender.

### "vrpn_Tracker Acceleration"

This message contains the current linear and angular acceleration of a single
sensor on the tracker.
The message body consists of a 32-bit signed integer representing the sensor id,
followed by 32-bit padding without meaning,
three 64-bit floating point values representing linear acceleration,
four 64-bit floating point values representing angular acceleration in quaternion form,
and a final 64-bit floating point representing the update time interval:

- `sensor` id (`i32`)
- padding (`i32`)
- `acc` (`[f64; 3]`)
- `acc_quat` (`[f64; 4]`)
- `acc_quat_dt` (`f64`)

### "vrpn_Tracker Pos_Quat"

This message contains the current position and orientation of a single sensor
on the tracker.
The message body consists of a 32-bit signed integer representing the sensor id,
followed by 32-bit padding without meaning,
three 64-bit floating point values representing position,
and four 64-bit floating point values representing orientation in quaternion form:

- `sensor` id (`i32`)
- padding (`i32`)
- `pos` (`[f64; 3]`)
- `quat` (`[f64; 4]`)

### "vrpn_Tracker Velocity"

This message contains the current linear and angular velocity of a single
sensor on the tracker.
The message body consists of a 32-bit signed integer representing the sensor id,
followed by 32-bit padding without meaning,
three 64-bit floating point values representing linear velocity,
and four 64-bit floating point values representing angular velocity in quaternion form:

- `sensor` id (`i32`)
- padding (`i32`)
- `vel` (`[f64; 3]`)
- `vel_quat` (`[f64; 4]`)

## Connection establishment modes

There are two basic network modes: TCP-only and UDP+TCP.

### TCP-only

This is the simpler mode.
The network client connects to the server over TCP (by default, port 3883),
and sends its magic cookie data.
It then receives the magic cookie data of the server.
If the cookies are compatible, the connection proceeds.

### UDP+TCP

This is somewhat more complex,
since the client actually opens a port that it tells the server about.

The client sends a datagram to the server (UDP, default port 3883) with the following:

    127.0.0.1 51221

where `127.0.0.1` is replaced by the (IPv4, in the mainline C++ codebase)
server-facing IP address of the client,
and `51221` is the TCP port number that the client is listening on.

The message is null-terminated.

TODO: On the trace I gathered, the message was 16 bytes.
Investigate if this is padded to `vrpn_ALIGN` or just simply null-terminated.

The server then connects to the client on the indicated port, over TCP,
and sends its magic cookie data.
If the data is acceptable to the client, it replies with its own magic cookie,
as in the TCP-only case, and the connection proceeds.

### Magic cookie data

This is a version stamp used to verify compatibility.
It is exchanged bidirectionally during the handshake in both modes.
Normally, only the major version must match.
Additionally, a decimal digit (in the place of `M`) conveys the desired
"remote logging mode" from the other end: it's a bitmask,
where 1 is incoming and 2 is outgoing
(so 0 is no remote logging and 3 is both incoming and outgoing logging).

    vrpn: ver. 07.35  M

Note the two spaces between the version number and the logging mode.
Additionally, this is packed out to 24 bytes.

## Sender description message

Message fields are as follows:

- The sender ID contains the sender ID being described.
- The message type ID is `-1` (`vrpn_CONNECTION_SENDER_DESCRIPTION`)

The message body contains the following fields:

- `length` of incoming sender identifier (`u32`) - this includes the null terminator.
- The incoming sender identifier, plus a null-terminator byte.

## Message type description message

Message fields are as follows:

- The sender ID contains the message type ID being described.
- The message type ID is `-2` (`vrpn_CONNECTION_TYPE_DESCRIPTION`)

The message body contains the following fields:

- `length` of incoming message type identifier (`u32`) - this includes the null terminator.
- The incoming message type identifier, plus a null-terminator byte.

## UDP description message

In the UDP+TCP connection mode,
the client notifies the server not just of the TCP callback port,
but also of the UDP callback port.
Unlike the TCP callback port
(which is carried in an un-framed message during handshake),
the UDP callback port is conveyed in a normally-framed message.

Message fields are as follows:

- The message body is the (null-terminated) IP address the server can send to.
- The sender ID is the port number.
- The message type ID is `-3` (`vrpn_CONNECTION_UDP_DESCRIPTION`)
- The "reliable" class of service is of course used
  (since even in UDP+TCP mode,
  there will be no low-latency/UDP channel established at this point of connection)

## Log description message

If there are any remote logging modes enabled,
the desired log names are sent.
This message's body contains the following fields:

- `strlen` of incoming log name (`i32`) - this excludes the null terminator.
- `strlen` of outgoing log name (`i32`) - this excludes the null terminator.
- The incoming log name (without null), plus a null terminator byte.
- The outgoing log name, plus a null-terminator byte.

This is wrapped in the normal message framing, with:

- system message type `-4` (`vrpn_CONNECTION_LOG_DESCRIPTION`)
- the remote log mode bitmask as the sender ID
- with "reliable" class of service (on the TCP channel)

---

## Copyright and License for this Protocol.md file

For this file only:

> Initially written by Ryan Pavlik. Copyright 2018 Collabora, Ltd.
>
> SPDX-License-Identifier: CC-BY-4.0

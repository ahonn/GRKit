# Custom BLE protocol

The `BLESLOTS` module (`examples/custom-slots`) lets a phone read and write Custom 1 and Custom 2 on GR III and GR IIIx.
The data is the 56-byte GR IV custom Image Control block. A name has up to 32 bytes.

The requests use the native Wi-Fi Passphrase characteristic, `0F38279C-FE9E-461B-8596-81287E8C9A81`.
The native setter refuses a value that starts with ESC, so a request changes nothing on a camera without GRKit.
The pairing and encryption rules of the camera apply.

## Requests

The client writes one frame with one ATT Write Request. Then it reads the response from the same characteristic.
A frame has at most 63 bytes. The negotiated ATT MTU must be at least 66, because the native reader always sends 64 bytes.

| Offset | Bytes | Field |
| --- | --- | --- |
| 0 | 5 | ESC and `GRK1` |
| 5 | 8 | Transaction ID in hex, not 0 |
| 13 | 1 | Action |
| 14 | 2 | Index in hex |
| 16 | 0 to 47 | Body |

| Action | Index | Body | Effect |
| --- | --- | --- | --- |
| `P` | `00` | None | Probe, and reserve the transaction |
| `B` | `00` | First fragment | Start a write command |
| `A` | Character offset | Next fragment | Append to the command |
| `X` | Command length | CRC, 4 hex digits | Run the command one time |
| `Q` | Page | `Rn` or `Mn` | Read a page of the data (`00` to `06`) or the name (`00` to `03`) of slot n |
| `Z` | `00` | None | Close the transaction. There is no response. |

A write command is `Wn` and 112 hex digits for a data block, or `Nn` and 0 to 64 hex digits for a name. Slot n is 1 or 2.
The camera changes nothing until it receives a valid `X`.
The first `Q` takes a snapshot, so all pages of one read come from the same value.

## Responses

| Offset | Bytes | Field |
| --- | --- | --- |
| 0 | 16 | The request header |
| 16 | 2 | Status in hex |
| 18 | 0 or 16 | Data in hex, for a `Q` |
| 18 or 34 | 4 | CRC in hex |

The response CRC covers all bytes before it. The commit CRC covers the complete command.
Both use CRC-16/CCITT with polynomial `0x1021` and initial value `0xFFFF`.

| Status | Meaning |
| --- | --- |
| `00` | Done |
| `01` | Invalid request |
| `02` | The module is off |
| `03` | Data write in a U mode |
| `04` | Invalid data block |
| `05` | An effect that GR III does not have |
| `06` | A value that a GR III Custom set cannot hold |
| `07` | The stored set cannot be encoded |
| `08` | The set is stored, but the camera did not apply it |
| `09` | Another transaction is active |
| `0A` | A repeated fragment has different bytes |

After status `08`, the set is changed. After each other failure, the set stays as it was.

## Transactions

One transaction runs at a time. It expires 30 seconds after the last request that advanced it.
If the module or GRKit is turned off, the transaction ends.

To do an operation, the client does these steps:

1. Make sure that the MTU is at least 66.
2. Send `P` with a new transaction ID.
3. Send the write fragments and `X`, or the `Q` pages.
4. Check each response: its length, the echoed header, and the CRC.
5. Send `Z`.

If a response is not valid, discard it and do not log it. After the transaction expires, the read gives the native Wi-Fi password.
An HTTP read of the password during a transaction can also get a protocol response.
`examples/custom-slots/client.py` does these steps.

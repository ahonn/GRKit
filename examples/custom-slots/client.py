"""Custom 1/2 over the native GR III Passphrase BLE characteristic.

docs/custom-ble-protocol.md gives the frame format. The camera needs the
BLESLOTS module of this folder (grkit module build) and the matching runtime.
The CLI needs the bleak package; see --help.
"""
import argparse
import asyncio
import binascii
import secrets

PASSPHRASE = "0F38279C-FE9E-461B-8596-81287E8C9A81"
PREFIX = b"\x1bGRK1"
MIN_MTU = 66


class ProtocolError(Exception):
    """The response cannot be attributed to this request."""


class CustomError(Exception):
    def __init__(self, status):
        self.status = status
        detail = " (stored, but live-view refresh failed)" if status == 8 else ""
        super().__init__(f"Custom operation refused: {status}{detail}")


def crc(data):
    return f"{binascii.crc_hqx(data, 0xffff):04x}".encode("ascii")


class CustomClient:
    """Use an encrypted Bleak-compatible connection, serialized per instance.

    Callers must also serialize other Passphrase users on the connection.
    Responses are never included in exceptions: an expired reply can be the
    native Wi-Fi password. No Wi-Fi connection is required.
    """

    def __init__(self, connection):
        self.connection = connection
        self.lock = asyncio.Lock()
        self.transaction = None

    def _header(self, action, index=0):
        return PREFIX + f"{self.transaction:08x}".encode() + action + f"{index:02x}".encode()

    async def _exchange(self, action, index=0, body=b"", data=False):
        header = self._header(action, index)
        request = header + body
        if self.connection.mtu_size < MIN_MTU or len(request) > min(63, self.connection.mtu_size - 3):
            raise ProtocolError("ATT MTU is too small for the request")
        # A retry uses the same transaction, offset, and contents. The server
        # recognizes duplicate fragments and commits without applying twice.
        for attempt in range(2):
            await self.connection.write_gatt_char(PASSPHRASE, request, response=True)
            raw = bytes(await self.connection.read_gatt_char(PASSPHRASE))
            reply = raw.split(b"\0", 1)[0]
            valid = (
                len(reply) in (22, 38)
                and reply[:16] == header
                and all(byte in b"0123456789abcdefABCDEF" for byte in reply[16:])
                and crc(reply[:-4]) == reply[-4:].lower()
            )
            if valid:
                try:
                    status = int(reply[16:18], 16)
                    payload = bytes.fromhex(reply[18:-4].decode("ascii"))
                except (ValueError, UnicodeError):
                    valid = False
                else:
                    valid = len(payload) == (8 if data and not status else 0)
            if valid:
                if status:
                    raise CustomError(status)
                return payload
            if attempt:
                raise ProtocolError("No matching Custom response; support or transaction is unavailable")

    async def _close(self):
        if self.transaction is not None:
            try:
                await self.connection.write_gatt_char(PASSPHRASE, self._header(b"Z"), response=True)
            finally:
                self.transaction = None

    async def close(self):
        async with self.lock:
            await self._close()

    async def _operation(self, callback):
        async with self.lock:
            if self.connection.mtu_size < MIN_MTU:
                raise ProtocolError("Custom BLE requires a negotiated ATT MTU of at least 66")
            self.transaction = secrets.randbelow(0xffffffff) + 1
            try:
                return await callback()
            finally:
                # A disconnected client cannot explicitly release its reply.
                # The RTOS lease is the fallback; cleanup must not mask errors.
                try:
                    await self._close()
                except Exception:
                    pass

    async def probe(self):
        await self._operation(lambda: self._exchange(b"P"))

    @staticmethod
    def _slot(slot):
        if slot not in (1, 2):
            raise ValueError("Custom slot must be 1 or 2")
        return str(slot).encode("ascii")

    async def _read(self, operation, slot, pages):
        body = operation + self._slot(slot)

        async def run():
            parts = []
            for page in range(pages):
                parts.append(await self._exchange(b"Q", page, body, data=True))
            return b"".join(parts)

        return await self._operation(run)

    async def read(self, slot):
        return await self._read(b"R", slot, 7)

    async def read_name(self, slot):
        return await self._read(b"M", slot, 4)

    async def _write(self, operation, slot, value):
        command = operation + self._slot(slot) + bytes(value).hex().encode("ascii")

        async def run():
            size = min(47, self.connection.mtu_size - 19)
            if size < 1:
                raise ProtocolError("ATT MTU is too small for Custom transfers")
            for offset in range(0, len(command), size):
                await self._exchange(b"B" if offset == 0 else b"A", offset, command[offset:offset + size])
            await self._exchange(b"X", len(command), crc(command))

        await self._operation(run)

    async def write(self, slot, block):
        if len(block) != 56:
            raise ValueError("Custom data must contain exactly 56 bytes")
        await self._write(b"W", slot, block)

    async def write_name(self, slot, name):
        if len(name) > 32:
            raise ValueError("Custom name must contain at most 32 bytes")
        await self._write(b"N", slot, name)


async def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("address", help="Camera identifier accepted by Bleak")
    parser.add_argument("action", choices=("probe", "read", "read-name", "write", "write-name"))
    parser.add_argument("--slot", type=int, choices=(1, 2), default=1)
    parser.add_argument("--value", help="Hex data for write, or UTF-8 text for write-name")
    args = parser.parse_args()
    if args.action.startswith("write") and args.value is None:
        parser.error("--value is required for a write")
    from bleak import BleakClient

    async with BleakClient(args.address) as connection:
        client = CustomClient(connection)
        if args.action == "probe":
            await client.probe()
            print("Custom BLE protocol v1 available")
        elif args.action == "read":
            print((await client.read(args.slot)).hex())
        elif args.action == "read-name":
            print((await client.read_name(args.slot)).rstrip(b"\0").decode("utf-8", errors="replace"))
        elif args.action == "write":
            await client.write(args.slot, bytes.fromhex(args.value))
        else:
            await client.write_name(args.slot, args.value.encode("utf-8"))


if __name__ == "__main__":
    asyncio.run(main())

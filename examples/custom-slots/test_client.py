"""Client checks without a Bluetooth adapter or third-party dependencies."""
import unittest

from client import CustomClient, CustomError, PREFIX, ProtocolError, crc


class Connection:
    def __init__(self, mtu=66):
        self.mtu_size = mtu
        self.requests = []
        self.status = 0
        self.corrupt = 0
        self.response = None
        self.closed = False

    async def write_gatt_char(self, uuid, value, response):
        assert response and len(value) <= self.mtu_size - 3
        self.requests.append(bytes(value))
        self.closed = value[13:14] == b"Z"

    async def read_gatt_char(self, uuid):
        if self.response is not None:
            return self.response
        request = self.requests[-1]
        body = request[:16] + f"{self.status:02x}".encode()
        if request[13:14] == b"Q" and not self.status:
            body += bytes([int(request[14:16], 16)] * 8).hex().encode()
        checksum = crc(body)
        if self.corrupt:
            self.corrupt -= 1
            checksum = b"xxxx"
        return (body + checksum).ljust(64, b"\0")


class ClientTests(unittest.IsolatedAsyncioTestCase):
    async def test_minimum_supported_mtu_write_reassembles_and_commits_then_closes(self):
        connection = Connection()
        value = bytes(range(56))
        await CustomClient(connection).write(2, value)
        fragments = connection.requests[:-2]
        command = b"W2" + value.hex().encode()
        self.assertEqual(b"".join(frame[16:] for frame in fragments), command)
        offset = 0
        for frame in fragments:
            self.assertEqual(int(frame[14:16], 16), offset)
            self.assertEqual(frame[13:14], b"B" if offset == 0 else b"A")
            offset += len(frame) - 16
        commit = connection.requests[-2]
        self.assertEqual(commit[13:], b"X72" + crc(command))
        self.assertTrue(connection.closed)
        self.assertEqual(len({frame[5:13] for frame in connection.requests}), 1)

    async def test_large_mtu_still_obeys_native_63_byte_limit(self):
        connection = Connection(128)
        await CustomClient(connection).write(1, bytes(56))
        self.assertEqual([len(frame) for frame in connection.requests], [63, 63, 36, 20, 16])

    async def test_read_uses_one_transaction_and_closes(self):
        connection = Connection()
        actual = await CustomClient(connection).read(1)
        self.assertEqual(actual, b"".join(bytes([page] * 8) for page in range(7)))
        self.assertTrue(connection.closed)
        self.assertEqual(len({frame[5:13] for frame in connection.requests}), 1)

    async def test_corrupt_reply_retries_identical_frame(self):
        connection = Connection()
        connection.corrupt = 1
        await CustomClient(connection).probe()
        self.assertEqual(connection.requests[0], connection.requests[1])
        self.assertTrue(connection.closed)

    async def test_unknown_reply_never_appears_in_error(self):
        connection = Connection()
        connection.response = b"private-native-password".ljust(64, b"\0")
        with self.assertRaises(ProtocolError) as caught:
            await CustomClient(connection).probe()
        self.assertNotIn("private-native-password", str(caught.exception))
        self.assertTrue(connection.closed)

    async def test_business_error_is_not_retried(self):
        connection = Connection()
        connection.status = 8
        with self.assertRaises(CustomError) as caught:
            await CustomClient(connection).probe()
        self.assertEqual(caught.exception.status, 8)
        self.assertIn("stored", str(caught.exception))
        self.assertEqual(len(connection.requests), 2)
        self.assertTrue(connection.closed)

    async def test_valid_crc_does_not_make_malformed_hex_a_response(self):
        class Malformed(Connection):
            async def read_gatt_char(self, uuid):
                body = self.requests[-1][:16] + self.suffix
                return body + crc(body)

        for suffix in (b" 0", b"+0", b"00" + b"00 " * 8, b"00" + b"gg" * 8):
            with self.subTest(suffix=suffix):
                connection = Malformed()
                connection.suffix = suffix
                with self.assertRaises(ProtocolError):
                    await CustomClient(connection).read(1)
                self.assertTrue(connection.closed)

    async def test_names_include_empty_and_full_byte_values(self):
        connection = Connection(128)
        client = CustomClient(connection)
        await client.write_name(1, b"")
        self.assertEqual(connection.requests[0][16:], b"N1")
        connection.requests.clear()
        name = bytes(range(32))
        await client.write_name(2, name)
        self.assertEqual(b"".join(frame[16:] for frame in connection.requests[:-2]), b"N2" + name.hex().encode())
        self.assertEqual(len(await client.read_name(2)), 32)

    async def test_invalid_inputs_do_not_touch_camera(self):
        connection = Connection()
        client = CustomClient(connection)
        for slot in (0, 3):
            with self.assertRaises(ValueError):
                await client.read(slot)
        with self.assertRaises(ValueError):
            await client.write(1, bytes(55))
        with self.assertRaises(ValueError):
            await client.write_name(1, bytes(33))
        self.assertEqual(connection.requests, [])

    async def test_small_mtu_is_rejected_before_any_camera_io(self):
        for mtu in (23, 64, 65):
            with self.subTest(mtu=mtu):
                connection = Connection(mtu)
                client = CustomClient(connection)
                for operation in (client.probe, lambda: client.read(1), lambda: client.write_name(1, b"name")):
                    with self.assertRaisesRegex(ProtocolError, "at least 66"):
                        await operation()
                self.assertEqual(connection.requests, [])


if __name__ == "__main__":
    unittest.main()

//! The Linux producer of the mailbox to the App task
//! (runtime/control_mailbox.h), and the discovery of that mailbox.
use anyhow::{bail, ensure, Context, Result};
use grkit_core::abi::*;
use serde::Deserialize;
use std::{
    path::Path,
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Descriptor {
    pub header_address: u32,
    pub mailbox_address: u32,
    pub mailbox_size: u32,
    pub mailbox_version: u32,
    pub epoch: [u32; 4],
}
impl Descriptor {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= 4096, "Control descriptor exceeds its limit");
        let descriptor: Self =
            serde_json::from_slice(bytes).context("Invalid control descriptor")?;
        descriptor.validate()?;
        Ok(descriptor)
    }
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.mailbox_address & 63 == 0,
            "The control mailbox is not cache-line aligned"
        );
        ensure!(
            self.mailbox_size as usize == CONTROL_SIZE
                && self.mailbox_version == CONTROL_VERSION_VALUE,
            "Unsupported control mailbox layout"
        );
        for (address, size) in [
            (self.header_address, HOST_HEADER_SIZE as u32),
            (self.mailbox_address, self.mailbox_size),
        ] {
            ensure!(
                address & 3 == 0
                    && address >= 0x5100_0000
                    && address
                        .checked_add(size)
                        .is_some_and(|end| end <= 0x6000_0000),
                "Control descriptor is outside RTOS memory"
            );
        }
        Ok(())
    }
}

/// Implementations provide coherent volatile words. `sync` must synchronize
/// cache aliases as well as CPU/compiler ordering, not merely issue a fence.
pub trait MailboxIo {
    fn sync(&mut self) -> Result<()>;
    fn read(&self, offset: usize) -> u32;
    fn write(&mut self, offset: usize, value: u32);
}

/// The answer of the App task to one command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Reply {
    pub result: u32,
    pub bytes: Vec<u8>,
}

const POLL: Duration = Duration::from_millis(10);

/// The one producer of the mailbox. Each command is a single exchange that
/// the App task answers in its next poll (100 ms). The commands only read
/// camera state or set a choice to a given value, so a command whose answer
/// did not arrive can be sent again.
pub struct Client<M: MailboxIo> {
    memory: M,
}
impl<M: MailboxIo> Client<M> {
    pub fn new(mut memory: M) -> Result<Self> {
        memory.sync()?;
        ensure!(
            memory.read(CONTROL_MAGIC) == CONTROL_MAGIC_VALUE
                && memory.read(CONTROL_VERSION) == CONTROL_VERSION_VALUE
                && memory.read(CONTROL_CAPACITY) as usize == CONTROL_PAYLOAD_BYTES,
            "Unsupported control mailbox"
        );
        Ok(Self { memory })
    }
    /// The count of setting changes, which the App task keeps in its own
    /// cache line and cleans to memory on each change. This side maps that
    /// line without a cache, so the read needs no `sync`.
    pub fn state_changes(&self) -> u32 {
        self.memory.read(CONTROL_STATE_CHANGES)
    }
    /// Mark the service as running or stopped in its own line of the
    /// mailbox, where the resident loader of the camera reads it.
    pub fn mark_service(&mut self, running: bool) -> Result<()> {
        let value = if running { CONTROL_SERVICE_RUNNING } else { 0 };
        self.memory.write(CONTROL_SERVICE, value);
        self.memory.sync()
    }
    fn epoch(&self) -> [u32; 4] {
        std::array::from_fn(|i| self.memory.read(CONTROL_EPOCH + i * 4))
    }
    /// The request, ready, and release markers, which only step by one.
    fn markers(&mut self) -> Result<(u32, u32, u32)> {
        self.memory.sync()?;
        let (request, ready, release) = (
            self.memory.read(CONTROL_REQUEST),
            self.memory.read(CONTROL_READY),
            self.memory.read(CONTROL_RELEASE),
        );
        ensure!(
            request <= CONTROL_SEQUENCE_MAX
                && ready <= request
                && request - ready <= 1
                && release <= ready
                && ready - release <= 1,
            "Invalid control markers"
        );
        Ok((request, ready, release))
    }
    /// Free the slot: wait for the answer to a request that an earlier
    /// service left, and release it. False when no answer came in time.
    fn settle(&mut self, deadline: Instant) -> Result<bool> {
        loop {
            let (request, ready, release) = self.markers()?;
            if request == ready {
                if release != ready {
                    self.memory.write(CONTROL_RELEASE, ready);
                    self.memory.sync()?;
                }
                return Ok(true);
            }
            if Instant::now() >= deadline {
                return Ok(false);
            }
            std::thread::sleep(POLL);
        }
    }
    fn submit(&mut self, epoch: [u32; 4], command: u32, payload: &[u8]) -> Result<u32> {
        ensure!(
            payload.len() <= CONTROL_PAYLOAD_BYTES && payload.len().is_multiple_of(4),
            "Invalid control payload"
        );
        let (request, ..) = self.markers()?;
        ensure!(
            request < CONTROL_SEQUENCE_MAX,
            "The control sequence is used up until the camera starts again"
        );
        for (i, word) in epoch.iter().enumerate() {
            self.memory.write(CONTROL_REQUEST_EPOCH + i * 4, *word);
        }
        self.memory.write(CONTROL_COMMAND, command);
        self.memory.write(CONTROL_LENGTH, payload.len() as u32);
        self.memory
            .write(CONTROL_CHECKSUM, crc32fast::hash(payload));
        for (i, bytes) in payload.as_chunks::<4>().0.iter().enumerate() {
            self.memory
                .write(CONTROL_PAYLOAD + i * 4, u32::from_le_bytes(*bytes));
        }
        self.memory.sync()?;
        self.memory.write(CONTROL_REQUEST, request + 1);
        self.memory.sync()?;
        Ok(request + 1)
    }
    /// The answer to request `sequence`, released, or None at the deadline.
    fn answer(&mut self, sequence: u32, deadline: Instant) -> Result<Option<Reply>> {
        loop {
            let (request, ready, _) = self.markers()?;
            ensure!(request == sequence, "Another producer used the mailbox");
            if ready == sequence {
                break;
            }
            if Instant::now() >= deadline {
                return Ok(None);
            }
            std::thread::sleep(POLL);
        }
        let length = self.memory.read(CONTROL_RESPONSE_LENGTH) as usize;
        ensure!(
            length <= CONTROL_PAYLOAD_BYTES,
            "The control response exceeds its limit"
        );
        let mut bytes = Vec::with_capacity(length.next_multiple_of(4));
        for offset in (0..length).step_by(4) {
            bytes.extend(self.memory.read(CONTROL_RESPONSE + offset).to_le_bytes());
        }
        bytes.truncate(length);
        ensure!(
            crc32fast::hash(&bytes) == self.memory.read(CONTROL_RESPONSE_CHECKSUM),
            "The control response is corrupt"
        );
        let result = self.memory.read(CONTROL_RESULT);
        self.memory.write(CONTROL_RELEASE, sequence);
        self.memory.sync()?;
        Ok(Some(Reply { result, bytes }))
    }
    /// Send one command and wait for its answer: None when the App task did
    /// not answer within `timeout`. The first command of a start of the
    /// camera installs `nonce` as the epoch; a later service keeps that
    /// epoch.
    pub fn exchange(
        &mut self,
        nonce: [u8; 16],
        command: u32,
        payload: &[u8],
        timeout: Duration,
    ) -> Result<Option<Reply>> {
        let deadline = Instant::now() + timeout;
        if !self.settle(deadline)? {
            return Ok(None);
        }
        if self.epoch() == [0; 4] {
            ensure!(nonce != [0; 16], "The attach nonce must be nonzero");
            let sequence = self.submit([0; 4], CONTROL_ATTACH, &nonce)?;
            match self.answer(sequence, deadline)? {
                Some(Reply { result: 0, .. }) => {}
                Some(reply) => bail!("The runtime rejected the attach: {}", reply.result),
                None => return Ok(None),
            }
            ensure!(self.epoch() != [0; 4], "The runtime kept no epoch");
        }
        let sequence = self.submit(self.epoch(), command, payload)?;
        self.answer(sequence, deadline)
    }
}

/// Find the mailbox with a core of the package folder of the card. The card
/// starts those cores at each power-on, so this runs no file that the camera
/// did not already run.
#[cfg(all(target_os = "linux", target_arch = "arm"))]
pub fn discover(root: &Path) -> Result<Client<MappedMailbox>> {
    platform::discover(root)
}
#[cfg(not(all(target_os = "linux", target_arch = "arm")))]
pub fn discover(_root: &Path) -> Result<Client<MappedMailbox>> {
    bail!("Runtime control is available only on the camera's ARM Linux")
}
#[cfg(all(target_os = "linux", target_arch = "arm"))]
pub use platform::MappedMailbox;
#[cfg(not(all(target_os = "linux", target_arch = "arm")))]
pub struct MappedMailbox {
    _private: (),
}
#[cfg(not(all(target_os = "linux", target_arch = "arm")))]
impl MailboxIo for MappedMailbox {
    fn sync(&mut self) -> Result<()> {
        bail!("Runtime control is unavailable on this platform")
    }
    fn read(&self, _offset: usize) -> u32 {
        unreachable!("no mapping can be created on this platform")
    }
    fn write(&mut self, _offset: usize, _value: u32) {
        unreachable!("no mapping can be created on this platform")
    }
}

// Match runtime::core_path_of. I*.BIN files are RTOS native loader images,
// not Linux installers, and must never be executed for discovery.
#[cfg(any(test, all(target_os = "linux", target_arch = "arm")))]
fn installer_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes == b"CORE.BIN"
        || (bytes.len() == 9
            && &bytes[..4] == b"CORE"
            && (b'1'..=b'9').contains(&bytes[4])
            && &bytes[5..] == b".BIN")
}

#[cfg(all(target_os = "linux", target_arch = "arm"))]
mod platform {
    use super::*;
    use std::{
        fs::{self, File, OpenOptions},
        io::{Read, Write},
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::OpenOptionsExt,
        },
        process::{Command, Stdio},
        thread,
        time::{Duration, Instant},
    };
    const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(2);
    const MAX_INSTALLERS: usize = 9;
    const MAX_ENTRIES: usize = 128;
    const MAX_BINARY: u64 = 8 * 1024 * 1024;
    pub struct MappedMailbox {
        mapping: *mut libc::c_void,
        length: usize,
        offset: usize,
        _lock: File,
    }
    // SAFETY: moving the mapping transfers its sole Linux producer and flock
    // ownership. Access still requires &mut Client; no Sync implementation is
    // provided, and all raw addresses remain private to this value.
    unsafe impl Send for MappedMailbox {}

    impl Drop for MappedMailbox {
        fn drop(&mut self) {
            // SAFETY: this mapping and its exact length belong to this value.
            unsafe {
                libc::munmap(self.mapping, self.length);
            }
        }
    }
    impl MappedMailbox {
        fn map(descriptor: &Descriptor, lock: File) -> Result<Self> {
            descriptor.validate()?;
            // SAFETY: sysconf reads a constant process property.
            let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
            ensure!(
                page > 0 && (page as usize).is_power_of_two(),
                "Unsupported memory page size"
            );
            let page = page as usize;
            let physical = descriptor.mailbox_address as usize;
            let base = physical & !(page - 1);
            let offset = physical - base;
            let length = (offset + CONTROL_SIZE).div_ceil(page) * page;
            let device = OpenOptions::new()
                .read(true)
                .write(true)
                .custom_flags(libc::O_SYNC)
                .open("/dev/mem")?;
            // SAFETY: a validated physical mailbox range is page aligned and
            // bounded. We retain this mapping until Drop and never expose it.
            let mapping = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    length,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_SHARED,
                    device.as_raw_fd(),
                    base as libc::off_t,
                )
            };
            ensure!(
                mapping != libc::MAP_FAILED,
                "Map runtime control mailbox: {}",
                std::io::Error::last_os_error()
            );
            Ok(Self {
                mapping,
                length,
                offset,
                _lock: lock,
            })
        }
        fn word(&self, offset: usize) -> *mut u32 {
            assert!(offset.is_multiple_of(4) && offset <= CONTROL_SIZE - 4);
            // SAFETY: the checked word falls wholly within the owned mapping.
            unsafe { self.mapping.cast::<u8>().add(self.offset + offset).cast() }
        }
    }
    impl MailboxIo for MappedMailbox {
        fn sync(&mut self) -> Result<()> {
            // Match INSTALL's alias synchronization. A successful PING is still
            // required to prove this target's cross-OS transport is responsive.
            unsafe {
                core::arch::asm!("dsb sy", options(nostack, preserves_flags));
            }
            let base = self.mapping as usize + self.offset;
            // O_SYNC /dev/mem maps the Linux view without normal data caching.
            // RTOS cleans descriptor and consumer lines to PoC. Never flush a
            // Linux alias across those RTOS-owned cache lines: only the producer
            // metadata and payload belong to this side of the protocol.
            for (start, end) in [
                (CONTROL_REQUEST, CONTROL_CLAIMED),
                (CONTROL_PAYLOAD, CONTROL_RESPONSE),
            ] {
                // SAFETY: these generated ranges are within this mapping; ARM
                // cacheflush accepts mapped virtual start/end addresses and 0.
                let result = unsafe { libc::syscall(0x0f0002, base + start, base + end, 0usize) };
                ensure!(
                    result == 0,
                    "Synchronize control producer alias: {}",
                    std::io::Error::last_os_error()
                );
            }
            unsafe {
                core::arch::asm!("dsb sy", "dmb ish", options(nostack, preserves_flags));
            }
            Ok(())
        }
        fn read(&self, offset: usize) -> u32 {
            // SAFETY: word checks alignment and bounds; firmware owns its data.
            unsafe { std::ptr::read_volatile(self.word(offset)) }
        }
        fn write(&mut self, offset: usize, value: u32) {
            // SAFETY: word checks alignment and bounds. Client writes only the
            // producer-owned fields after checking REQUEST/READY/RELEASE.
            unsafe {
                std::ptr::write_volatile(self.word(offset), value);
            }
        }
    }
    fn lock() -> Result<File> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open("/tmp/grkit-control.lock")?;
        ensure!(file.metadata()?.is_file(), "Invalid control producer lock");
        // SAFETY: flock operates on the live owned file descriptor.
        ensure!(
            unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0,
            "Another runtime control producer is active"
        );
        Ok(file)
    }
    pub(super) fn discover(root: &Path) -> Result<Client<MappedMailbox>> {
        let lock = lock()?;
        let directory = root.join(grkit_core::card::PACKAGE_FOLDER);
        let mut candidates = Vec::new();
        for (count, entry) in fs::read_dir(directory)?.enumerate() {
            ensure!(
                count < MAX_ENTRIES,
                "Package discovery entry limit exceeded"
            );
            let entry = entry?;
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| anyhow::anyhow!("Invalid package filename"))?;
            if !installer_name(&name) {
                continue;
            }
            ensure!(
                entry.file_type()?.is_file(),
                "Control installer is not a regular file"
            );
            ensure!(
                candidates.len() < MAX_INSTALLERS,
                "Control installer count limit exceeded"
            );
            candidates.push((name, entry.path()));
        }
        candidates.sort();
        // The core of another build can also find the mailbox, by
        // relocation. INSTALL.SH records the core that installed: try it
        // first. A start with the fast path records none.
        if let Some(installed) = installed_core(root) {
            candidates.sort_by_key(|(name, _)| *name != installed);
        }
        let mut failures = Vec::new();
        for (name, path) in candidates {
            // O_NOFOLLOW closes the directory-entry/open race for symlinks.
            let file = OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NOFOLLOW)
                .open(path)?;
            ensure!(
                file.metadata()?.is_file(),
                "Control installer is not a regular file"
            );
            let mut bytes = Vec::new();
            file.take(MAX_BINARY + 1).read_to_end(&mut bytes)?;
            ensure!(
                bytes.len() as u64 <= MAX_BINARY,
                "Control installer is oversized"
            );
            match run_installer(&bytes) {
                Ok(descriptor) => {
                    return Client::new(MappedMailbox::map(&descriptor, lock)?);
                }
                Err(error) => failures.push(format!("{name}: {error:#}")),
            }
        }
        bail!(
            "No installer found a live runtime control mailbox: {}",
            if failures.is_empty() {
                "no CORE files".into()
            } else {
                failures.join("; ")
            }
        )
    }
    /// The core file that INSTALL.SH last installed with, from
    /// `GRKIT/CORE.TXT`. A start with the fast path does not write it.
    fn installed_core(root: &Path) -> Option<String> {
        let mut text = String::new();
        File::open(
            root.join(grkit_core::card::DATA_FOLDER_PATH)
                .join(grkit_core::runtime::CORE_CHOICE),
        )
        .ok()?
        .take(64)
        .read_to_string(&mut text)
        .ok()?;
        let name = text.split_whitespace().next()?;
        installer_name(name).then(|| name.to_owned())
    }
    fn run_installer(bytes: &[u8]) -> Result<Descriptor> {
        // Execute the verified RAM copy, so replacing a card file after hashing
        // cannot replace the executable or keep the card executable mapped.
        let fd = unsafe { libc::memfd_create(c"grkit-control-info".as_ptr(), libc::MFD_CLOEXEC) };
        ensure!(
            fd >= 0,
            "Create control discovery executable: {}",
            std::io::Error::last_os_error()
        );
        // SAFETY: fd is new and exclusively owned.
        let mut executable = unsafe { File::from_raw_fd(fd) };
        executable.write_all(bytes)?;
        let mut child = Command::new(format!("/proc/self/fd/{fd}"))
            .arg("--control-info")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let mut stdout = child.stdout.take().context("Missing discovery stdout")?;
        // SAFETY: stdout's descriptor remains alive while its flags are changed.
        let flags = unsafe { libc::fcntl(stdout.as_raw_fd(), libc::F_GETFL) };
        if flags < 0
            || unsafe { libc::fcntl(stdout.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK) }
                < 0
        {
            let _ = child.kill();
            let _ = child.wait();
            bail!("Configure discovery output");
        }
        let deadline = Instant::now() + DISCOVERY_TIMEOUT;
        let mut output = Vec::new();
        let mut buffer = [0; 512];
        let result = loop {
            if Instant::now() >= deadline {
                break Err(anyhow::anyhow!("Control discovery timed out"));
            }
            match stdout.read(&mut buffer) {
                Ok(0) => match child.try_wait() {
                    Ok(Some(status)) if status.success() => break Descriptor::parse(&output),
                    Ok(Some(status)) => {
                        break Err(anyhow::anyhow!(
                            "Installer exited with {status}; output: {}",
                            String::from_utf8_lossy(&output)
                        ));
                    }
                    Ok(None) => {}
                    Err(error) => break Err(error.into()),
                },
                Ok(count) => {
                    output.extend_from_slice(&buffer[..count]);
                    if output.len() > 4096 {
                        break Err(anyhow::anyhow!("Control discovery output limit exceeded"));
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => break Err(error.into()),
            }
            thread::sleep(Duration::from_millis(5));
        };
        let _ = child.kill();
        let _ = child.wait();
        result
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    /// A command of the test: its result and its response.
    pub(crate) type Handler = Box<dyn FnMut(u32, &[u8]) -> (u32, Vec<u8>) + Send>;
    /// The mailbox with a consumer that answers in `sync`, as the poll of
    /// the App task does between two reads of the service.
    pub(crate) struct Memory {
        pub words: Vec<u32>,
        pub stalled: bool,
        pub fail_sync: bool,
        pub handler: Handler,
        pub commands: Vec<u32>,
    }
    impl Memory {
        pub fn new() -> Self {
            let mut memory = Self {
                words: vec![0; CONTROL_WORDS],
                stalled: false,
                fail_sync: false,
                handler: Box::new(|_, _| (CONTROL_UNSUPPORTED, Vec::new())),
                commands: Vec::new(),
            };
            memory.words[CONTROL_MAGIC / 4] = CONTROL_MAGIC_VALUE;
            memory.words[CONTROL_VERSION / 4] = CONTROL_VERSION_VALUE;
            memory.words[CONTROL_CAPACITY / 4] = CONTROL_PAYLOAD_BYTES as u32;
            memory
        }
        fn bytes(&self, at: usize, length: usize) -> Vec<u8> {
            let mut bytes: Vec<u8> = self.words[at / 4..(at + length).div_ceil(4)]
                .iter()
                .flat_map(|word| word.to_le_bytes())
                .collect();
            bytes.truncate(length);
            bytes
        }
        fn consume(&mut self) {
            let request = self.words[CONTROL_REQUEST / 4];
            if self.stalled
                || request == self.words[CONTROL_READY / 4]
                || self.words[CONTROL_READY / 4] != self.words[CONTROL_RELEASE / 4]
            {
                return;
            }
            let command = self.words[CONTROL_COMMAND / 4];
            let payload = self.bytes(CONTROL_PAYLOAD, self.words[CONTROL_LENGTH / 4] as usize);
            let epoch = self.words[CONTROL_EPOCH / 4..CONTROL_EPOCH / 4 + 4].to_vec();
            let attached = epoch.iter().any(|word| *word != 0);
            self.commands.push(command);
            let (result, response) = if crc32fast::hash(&payload)
                != self.words[CONTROL_CHECKSUM / 4]
            {
                (CONTROL_BAD_DIGEST, Vec::new())
            } else if self.words[CONTROL_REQUEST_EPOCH / 4..CONTROL_REQUEST_EPOCH / 4 + 4] != epoch
            {
                (CONTROL_BAD_CONTEXT, Vec::new())
            } else if command == CONTROL_ATTACH {
                if attached {
                    (CONTROL_BAD_CONTEXT, Vec::new())
                } else {
                    for (i, bytes) in payload.as_chunks::<4>().0.iter().enumerate() {
                        self.words[CONTROL_EPOCH / 4 + i] = u32::from_le_bytes(*bytes);
                    }
                    (0, Vec::new())
                }
            } else if !attached {
                (CONTROL_BAD_CONTEXT, Vec::new())
            } else if command == CONTROL_PING {
                (0, Vec::new())
            } else {
                (self.handler)(command, &payload)
            };
            let mut padded = response.clone();
            padded.resize(response.len().next_multiple_of(4), 0);
            for (i, bytes) in padded.as_chunks::<4>().0.iter().enumerate() {
                self.words[CONTROL_RESPONSE / 4 + i] = u32::from_le_bytes(*bytes);
            }
            self.words[CONTROL_RESPONSE_LENGTH / 4] = response.len() as u32;
            self.words[CONTROL_RESPONSE_CHECKSUM / 4] = crc32fast::hash(&response);
            self.words[CONTROL_RESULT / 4] = result;
            self.words[CONTROL_CLAIMED / 4] = request;
            self.words[CONTROL_READY / 4] = request;
        }
    }
    impl MailboxIo for Memory {
        fn sync(&mut self) -> Result<()> {
            ensure!(!self.fail_sync, "Injected cache sync failure");
            self.consume();
            Ok(())
        }
        fn read(&self, offset: usize) -> u32 {
            self.words[offset / 4]
        }
        fn write(&mut self, offset: usize, value: u32) {
            assert!(
                matches!(
                    offset,
                    CONTROL_REQUEST
                        | CONTROL_RELEASE
                        | CONTROL_COMMAND
                        | CONTROL_LENGTH
                        | CONTROL_CHECKSUM
                        | CONTROL_SERVICE
                ) || (CONTROL_REQUEST_EPOCH..CONTROL_REQUEST_EPOCH + 16).contains(&offset)
                    || (CONTROL_PAYLOAD..CONTROL_RESPONSE).contains(&offset),
                "producer wrote consumer field {offset}"
            );
            self.words[offset / 4] = value;
        }
    }
    const SHORT: Duration = Duration::from_millis(30);

    #[test]
    fn the_first_command_attaches_and_later_commands_keep_the_epoch() {
        let mut memory = Memory::new();
        memory.handler = Box::new(|command, payload| (0, [&[command as u8], payload].concat()));
        let mut client = Client::new(memory).unwrap();
        let reply = client.exchange([7; 16], 9, &[1, 2, 3, 4], SHORT).unwrap();
        assert_eq!(
            reply,
            Some(Reply {
                result: 0,
                bytes: vec![9, 1, 2, 3, 4]
            })
        );
        assert_eq!(client.epoch(), [0x07070707; 4]);
        // A restarted service has another nonce and keeps the epoch.
        let reply = client.exchange([8; 16], CONTROL_PING, &[], SHORT).unwrap();
        assert_eq!(reply.unwrap().result, 0);
        assert_eq!(client.epoch(), [0x07070707; 4]);
        assert_eq!(client.memory.commands, [CONTROL_ATTACH, 9, CONTROL_PING]);
        assert_eq!(client.memory.read(CONTROL_RELEASE), 3);
    }

    #[test]
    fn the_service_marks_itself_without_a_command() {
        let mut client = Client::new(Memory::new()).unwrap();
        client.mark_service(true).unwrap();
        assert_eq!(
            client.memory.words[CONTROL_SERVICE / 4],
            CONTROL_SERVICE_RUNNING
        );
        client.mark_service(false).unwrap();
        assert_eq!(client.memory.words[CONTROL_SERVICE / 4], 0);
        assert!(client.memory.commands.is_empty());
    }

    #[test]
    fn a_command_without_an_answer_times_out_and_the_next_one_recovers() {
        let mut client = Client::new(Memory::new()).unwrap();
        assert!(client.exchange([0; 16], CONTROL_PING, &[], SHORT).is_err());
        client.memory.stalled = true;
        assert_eq!(
            client.exchange([7; 16], CONTROL_PING, &[], SHORT).unwrap(),
            None
        );
        // The slot is still taken: nothing new is published.
        assert_eq!(client.memory.read(CONTROL_REQUEST), 1);
        assert_eq!(
            client.exchange([7; 16], CONTROL_PING, &[], SHORT).unwrap(),
            None
        );
        assert_eq!(client.memory.read(CONTROL_REQUEST), 1);
        // The App task polls again: the old request ends, then the new one.
        client.memory.stalled = false;
        let reply = client.exchange([7; 16], 5, &[], SHORT).unwrap().unwrap();
        assert_eq!(reply.result, CONTROL_UNSUPPORTED);
        assert_eq!(client.memory.commands, [CONTROL_ATTACH, 5]);
    }

    #[test]
    fn rejects_bad_layouts_payloads_markers_and_responses() {
        let mut memory = Memory::new();
        memory.words[CONTROL_VERSION / 4] += 1;
        assert!(Client::new(memory).is_err());
        let mut memory = Memory::new();
        memory.fail_sync = true;
        assert!(Client::new(memory).is_err());

        let mut client = Client::new(Memory::new()).unwrap();
        assert!(client.exchange([7; 16], 5, &[1], SHORT).is_err());
        let large = vec![0; CONTROL_PAYLOAD_BYTES + 4];
        assert!(client.exchange([7; 16], 5, &large, SHORT).is_err());
        client.memory.words[CONTROL_READY / 4] = 9;
        assert!(client.exchange([7; 16], 5, &[], SHORT).is_err());

        let mut memory = Memory::new();
        memory.handler = Box::new(|_, _| (0, vec![1, 2, 3]));
        let mut client = Client::new(memory).unwrap();
        assert_eq!(
            client
                .exchange([7; 16], 5, &[], SHORT)
                .unwrap()
                .unwrap()
                .bytes,
            [1, 2, 3]
        );
        client.memory.words[CONTROL_REQUEST / 4] = CONTROL_SEQUENCE_MAX;
        client.memory.words[CONTROL_READY / 4] = CONTROL_SEQUENCE_MAX;
        client.memory.words[CONTROL_RELEASE / 4] = CONTROL_SEQUENCE_MAX;
        assert!(client.exchange([7; 16], 5, &[], SHORT).is_err());
    }

    #[test]
    fn discovery_accepts_only_linux_core_installer_names() {
        for name in ["CORE.BIN", "CORE1.BIN", "CORE9.BIN"] {
            assert!(installer_name(name));
        }
        for name in [
            "I588221.BIN",
            "INSTALL.BIN",
            "CORE0.BIN",
            "CORE10.BIN",
            "COREA.BIN",
            "core.bin",
            "../CORE.BIN",
            "CORE1.BIN/evil",
            "CÖRE1.BIN",
        ] {
            assert!(!installer_name(name), "{name}");
        }
    }

    #[test]
    fn descriptor_rejects_version_length_alignment_range_and_unknown_fields() {
        let descriptor = Descriptor {
            header_address: 0x53000000,
            mailbox_address: 0x53010000,
            mailbox_size: CONTROL_SIZE as u32,
            mailbox_version: CONTROL_VERSION_VALUE,
            epoch: [0; 4],
        };
        descriptor.validate().unwrap();
        let mut invalid = descriptor.clone();
        invalid.mailbox_version += 1;
        assert!(invalid.validate().is_err());
        let mut invalid = descriptor.clone();
        invalid.mailbox_size += 4;
        assert!(invalid.validate().is_err());
        let mut invalid = descriptor.clone();
        invalid.mailbox_address += 1;
        assert!(invalid.validate().is_err());
        let mut invalid = descriptor.clone();
        invalid.mailbox_address = 0xfffff000;
        assert!(invalid.validate().is_err());
        let mut invalid = descriptor;
        invalid.header_address = 0;
        assert!(invalid.validate().is_err());
        assert!(Descriptor::parse(b"{\"shell\":\"anything\"}").is_err());
        assert!(Descriptor::parse(&vec![b' '; 4097]).is_err());
    }
}

package studio.techflow.grkit;

import android.Manifest;
import android.app.Activity;
import android.bluetooth.*;
import android.bluetooth.le.*;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.pm.PackageManager;
import android.net.*;
import android.net.wifi.WifiNetworkSpecifier;
import android.os.*;
import android.util.Base64;
import org.json.*;
import java.io.*;
import java.net.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.concurrent.*;

/** App-owned radio transport; the shared Rust state machine owns update policy. */
public final class CameraBridge {
    private static final int PERMISSIONS = 4701;
    private static final int SINGLE_PERMISSION = 4702;
    // Bluetooth connect: per-attempt budget, attempts, the delay between them,
    // and how many of them a native connect timeout may use.
    private static final long BLE_CONNECT_TIMEOUT_MS = 15_000;
    private static final int BLE_CONNECT_ATTEMPTS = 3;
    private static final long BLE_CONNECT_RETRY_DELAY_MS = 2_000;
    private static final int BLE_CONNECT_TIMEOUT_RETRIES = 1;
    /** The longest a read the camera refused for want of pairing waits for the system pairing dialog. */
    private static final long PAIRING_WAIT_MS = 90_000;
    /**
     * The longest a Wi-Fi join waits for the system to bring up the camera
     * network, in foreground time: the system activity that searches for the
     * network and asks the user covers the app, and that time does not count.
     */
    private static final long JOIN_TIMEOUT_MS = 30_000;
    private static final long JOIN_POLL_MS = 250;
    /** The activity is in front: no system dialog covers it. */
    private static volatile boolean resumed;
    // Forced reassociation: the least wait after dropping the camera network,
    // the poll interval, and the bound on the wait for the network to go away.
    private static final long REASSOCIATE_SETTLE_MIN_MS = 1_000;
    private static final long REASSOCIATE_SETTLE_POLL_MS = 500;
    private static final long REASSOCIATE_SETTLE_TIMEOUT_MS = 3_000;
    // Closing the camera access point after a cancel: taking the link back,
    // and each command on it.
    private static final long TEARDOWN_RECONNECT_TIMEOUT_MS = 6_000;
    private static final long TEARDOWN_COMMAND_TIMEOUT_MS = 2_000;
    /** Android 17 gates LAN traffic behind this permission for apps that target it. */
    private static final String LOCAL_NETWORK = "android.permission.ACCESS_LOCAL_NETWORK";
    private static final int LOCAL_NETWORK_API = 37;
    private static final long LOCAL_NETWORK_REQUEST_MS = 60_000;
    private static final UUID INFORMATION = uuid("9A5ED1C5-74CC-4C50-B5B6-66A48E7CCFF1");
    private static final UUID CONTROL = uuid("0F291746-0C80-4726-87A7-3C501FD3B4B6");
    private static final UUID POWER = uuid("B58CE84C-0666-4DE9-BEC8-2D27B27B3211");
    private static final UUID NETWORK = uuid("9111CDD0-9F01-45C4-A2D4-E09E8FB0424D");
    private static final UUID SSID = uuid("90638E5A-E77D-409D-B550-78F7E1CA5AB4");
    private static final UUID PASSWORD = uuid("0F38279C-FE9E-461B-8596-81287E8C9A81");
    private static final UUID CCCD = uuid("00002902-0000-1000-8000-00805f9b34fb");
    private static final Handler MAIN = new Handler(Looper.getMainLooper());
    private static final ExecutorService RADIO = Executors.newSingleThreadExecutor();
    private static final ExecutorService HTTP = Executors.newFixedThreadPool(3);
    private static volatile Activity activity;
    private static Context receiverContext;
    private static volatile Session session;
    private static volatile Network cameraNetwork;
    private static volatile CompletableFuture<Network> joining;
    /** The answer to a single permission request: true, false, or null when the dialog gave none. */
    private static volatile CompletableFuture<Boolean> permissionAnswer;
    private static final Map<Long, HttpURLConnection> requests = new ConcurrentHashMap<>();
    private static final Set<Long> activeRequests = ConcurrentHashMap.newKeySet();
    private static final Set<Long> cancelledRequests = ConcurrentHashMap.newKeySet();
    private static ConnectivityManager.NetworkCallback networkCallback;
    private static volatile long generation;
    private static boolean scanning;
    private static boolean wantsScan;
    /** A connect attempt is running; scanning stays paused until it ends. */
    private static volatile boolean connecting;
    private static boolean requestingPermissions;
    /** The user asked for a camera: the permission request may show. */
    private static boolean mayAsk;

    private CameraBridge() {}
    private static UUID uuid(String value) { return UUID.fromString(value); }
    private static native void complete(long id, String json, String error);
    private static native void event(String kind, String json);

    private static final BroadcastReceiver ADAPTER_STATE = new BroadcastReceiver() {
        @Override public void onReceive(Context context, Intent intent) {
            if (!BluetoothAdapter.ACTION_STATE_CHANGED.equals(intent.getAction())) return;
            int state = intent.getIntExtra(BluetoothAdapter.EXTRA_STATE, BluetoothAdapter.ERROR);
            if (state == BluetoothAdapter.STATE_ON) {
                resumeScan();
            } else if (state == BluetoothAdapter.STATE_OFF || state == BluetoothAdapter.STATE_TURNING_OFF) {
                stopScan();
                event("unavailable", "true");
            }
        }
    };

    public static void initialize(Activity host) {
        activity = host;
        if (receiverContext == null) {
            Context context = host.getApplicationContext();
            IntentFilter filter = new IntentFilter(BluetoothAdapter.ACTION_STATE_CHANGED);
            // Bluetooth broadcasts originate in a privileged process outside the app.
            // ACTION_STATE_CHANGED is a protected broadcast, not an app-supplied event.
            if (Build.VERSION.SDK_INT >= 33) {
                context.registerReceiver(ADAPTER_STATE, filter, Context.RECEIVER_EXPORTED);
            } else {
                context.registerReceiver(ADAPTER_STATE, filter);
            }
            receiverContext = context;
        }
    }

    public static void onResume() {
        resumed = true;
        resumeScan();
    }

    public static void onPause() {
        resumed = false;
    }

    private static void resumeScan() {
        // Returning from Settings may grant access. A denied permission must not
        // prompt again merely because the activity regained focus.
        if (!wantsScan || !radioPermissions()) return;
        // A connect attempt pauses scanning until it finishes or is cancelled.
        if (connecting) return;
        try { startScan(); }
        catch (SecurityException ignored) { event("unavailable", "true"); }
    }

    public static String remembered() {
        Activity host = activity;
        return host == null ? "null" : host.getSharedPreferences("camera", Context.MODE_PRIVATE)
                .getString("remembered", "null");
    }

    public static void saveRemembered(String json) {
        Activity host = activity;
        if (host == null || !host.getSharedPreferences("camera", Context.MODE_PRIVATE)
                .edit().putString("remembered", json).commit()) {
            throw new IllegalStateException("Camera preferences could not be saved");
        }
    }

    public static void execute(long id, String operation, String json) {
        if (operation.equals("cancelRequest")) {
            try {
                long original = new JSONObject(json).getLong("id");
                synchronized (activeRequests) {
                    if (activeRequests.contains(original)) cancelledRequests.add(original);
                }
                HttpURLConnection request = requests.remove(original);
                if (request != null) request.disconnect();
                complete(id, "null", null);
            } catch (JSONException error) { complete(id, null, "ConnectionLost"); }
            return;
        }
        if (operation.equals("network")) {
            // Answered at once: a radio operation in progress must not hold it.
            try { complete(id, networkSnapshot().toString(), null); }
            catch (Exception error) { complete(id, null, errorName(error)); }
            return;
        }
        if (operation.equals("askName")) {
            MAIN.post(() -> {
                try {
                    NamePrompt.show(activity, new JSONObject(json), name ->
                            complete(id, name == null ? "null" : JSONObject.quote(name), null));
                } catch (Exception error) { complete(id, "null", null); }
            });
            return;
        }
        if (operation.equals("scan") || operation.equals("stopScan") || operation.equals("cancel")
                || operation.equals("finish") || operation.equals("release")) {
            MAIN.post(() -> {
                try {
                    if (operation.equals("scan")) {
                        wantsScan = true;
                        if (new JSONObject(json).optBoolean("ask")) mayAsk = true;
                        startScan();
                    }
                    else if (operation.equals("stopScan")) { wantsScan = false; stopScan(); }
                    else if (operation.equals("finish")) finish();
                    else if (operation.equals("release")) releaseLink();
                    else cancel(new JSONObject(json).optBoolean("teardown"));
                    complete(id, "null", null);
                } catch (Exception error) { complete(id, null, errorName(error)); }
            });
            return;
        }
        long attempt = generation;
        if (operation.equals("http")) synchronized (activeRequests) { activeRequests.add(id); }
        (operation.equals("http") ? HTTP : RADIO).execute(() -> {
            try {
                JSONObject args = new JSONObject(json);
                Object result;
                switch (operation) {
                    case "setup": result = setup(args, attempt); break;
                    case "join": join(args, attempt); result = JSONObject.NULL; break;
                    case "endSession": endSession(args, attempt); result = JSONObject.NULL; break;
                    case "settings": openSettings(); result = JSONObject.NULL; break;
                    case "localNetwork": result = JSONObject.quote(localNetworkAccess(attempt)); break;
                    case "http": result = http(id, args); break;
                    default: throw new Failure("ConnectionLost");
                }
                complete(id, result.toString(), null);
            } catch (Exception error) { complete(id, null, errorName(error)); }
            finally {
                synchronized (activeRequests) { activeRequests.remove(id); cancelledRequests.remove(id); }
            }
        });
    }

    private static String errorName(Exception error) {
        Throwable cause = error;
        while (cause instanceof ExecutionException && cause.getCause() != null) cause = cause.getCause();
        if (cause instanceof Failure) return cause.getMessage();
        if (cause instanceof SecurityException) return "BluetoothUnavailable";
        return "ConnectionLost";
    }

    private static BluetoothAdapter adapter() {
        Activity host = activity;
        BluetoothManager manager = host == null ? null : host.getSystemService(BluetoothManager.class);
        return manager == null ? null : manager.getAdapter();
    }

    private static boolean granted(String permission) {
        return activity != null && activity.checkSelfPermission(permission) == PackageManager.PERMISSION_GRANTED;
    }

    private static boolean radioPermissions() {
        return Build.VERSION.SDK_INT >= 31
                ? granted(Manifest.permission.BLUETOOTH_SCAN) && granted(Manifest.permission.BLUETOOTH_CONNECT)
                : granted(Manifest.permission.ACCESS_FINE_LOCATION);
    }

    private static void startScan() {
        if (!radioPermissions()) {
            if (mayAsk && !requestingPermissions && activity != null) {
                requestingPermissions = true;
                List<String> required = new ArrayList<>();
                if (Build.VERSION.SDK_INT >= 31) {
                    required.add(Manifest.permission.BLUETOOTH_SCAN);
                    required.add(Manifest.permission.BLUETOOTH_CONNECT);
                }
                if (Build.VERSION.SDK_INT <= 32) required.add(Manifest.permission.ACCESS_FINE_LOCATION);
                if (Build.VERSION.SDK_INT >= 33) required.add(Manifest.permission.NEARBY_WIFI_DEVICES);
                activity.requestPermissions(required.toArray(new String[0]), PERMISSIONS);
            }
            return;
        }
        BluetoothAdapter adapter = adapter();
        boolean unavailable = adapter == null || !adapter.isEnabled();
        event("unavailable", Boolean.toString(unavailable));
        if (unavailable || scanning || !wantsScan) return;
        BluetoothLeScanner scanner = adapter.getBluetoothLeScanner();
        if (scanner == null) return;
        scanner.startScan(Arrays.asList(
                new ScanFilter.Builder().setServiceUuid(new ParcelUuid(INFORMATION)).build(),
                new ScanFilter.Builder().setServiceUuid(new ParcelUuid(CONTROL)).build()),
                new ScanSettings.Builder().setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY).build(), SCAN);
        scanning = true;
    }

    public static void onRequestPermissionsResult(int code, String[] permissions, int[] results) {
        if (code == SINGLE_PERMISSION) {
            CompletableFuture<Boolean> waiting = permissionAnswer;
            // No result means the dialog closed without an answer.
            if (waiting != null) waiting.complete(results.length == 0 ? null
                    : results[0] == PackageManager.PERMISSION_GRANTED);
            return;
        }
        if (code != PERMISSIONS) return;
        requestingPermissions = false;
        if (radioPermissions()) startScan();
        else event("unavailable", "true");
    }

    private static void stopScan() {
        if (!scanning) return;
        scanning = false;
        try {
            BluetoothAdapter adapter = adapter();
            if (adapter != null && adapter.getBluetoothLeScanner() != null)
                adapter.getBluetoothLeScanner().stopScan(SCAN);
        } catch (SecurityException ignored) { /* Permission can be revoked in Settings. */ }
    }

    private static final ScanCallback SCAN = new ScanCallback() {
        @Override public void onScanResult(int type, ScanResult result) {
            ScanRecord record = result.getScanRecord();
            if (record == null) return;
            try {
                byte[] data = record.getManufacturerSpecificData(0x065f);
                JSONArray manufacturer = new JSONArray();
                if (data != null) {
                    manufacturer.put(0x5f).put(0x06);
                    for (byte value : data) manufacturer.put(value & 255);
                }
                JSONArray services = new JSONArray();
                if (record.getServiceUuids() != null) for (ParcelUuid service : record.getServiceUuids()) {
                    services.put(service.toString());
                }
                // Rust reads power from each frame; freshness lives there.
                String id = result.getDevice().getAddress();
                event("scan", new JSONObject().put("id", id).put("name", record.getDeviceName())
                        .put("manufacturer", manufacturer).put("services", services)
                        .put("rssi", result.getRssi()).toString());
            } catch (JSONException | SecurityException ignored) { }
        }
        @Override public void onScanFailed(int error) {
            scanning = false;
            event("unavailable", "true");
        }
    };

    private static JSONObject setup(JSONObject args, long attempt) throws Exception {
        if (!radioPermissions()) throw new Failure("BluetoothUnavailable");
        BluetoothAdapter adapter = adapter();
        if (adapter == null || !adapter.isEnabled()) throw new Failure("BluetoothUnavailable");
        connecting = true;
        MAIN.post(CameraBridge::stopScan);
        pause(500, attempt);
        String id = args.getString("id");
        String model = args.getString("model");
        BluetoothDevice device;
        try { device = adapter.getRemoteDevice(id); }
        catch (IllegalArgumentException error) { throw new Failure("NotNearby"); }
        long connectTimeoutMs = args.optLong("connect_timeout_ms", BLE_CONNECT_TIMEOUT_MS);
        int timeoutRetries = args.optInt("timeout_retries", BLE_CONNECT_TIMEOUT_RETRIES);
        Session link = connectWithRetries(device, connectTimeoutMs, timeoutRetries, attempt);
        try {
            String ssid = credential(link.read(SSID));
            String password = credential(link.read(PASSWORD));
            if (ssid.isEmpty() || password.isEmpty()) throw new Failure("NotPaired");
            // Only a fresh advertisement skips the wake write.
            boolean alreadyOn = args.optBoolean("powered_on", false);
            if (!alreadyOn) {
                try { link.write(POWER, new byte[]{1}); }
                catch (Failure ignored) { /* Some firmware wakes before acknowledging. */ }
            }
            boolean ready = false;
            boolean subscribed = false;
            try { link.subscribe(); subscribed = true; }
            catch (Exception ignored) { /* Starting the AP does not require notification support. */ }
            try {
                for (int retry = 0; retry < 2 && !ready; retry++) {
                    link.write(NETWORK, new byte[]{1});
                    ready = subscribed && link.apReady.poll(2, TimeUnit.SECONDS) != null;
                    if (!ready && retry == 0) pause(500, attempt);
                }
            } catch (Exception ignored) {
                // AP startup can drop GATT. Credentials remain valid; HTTP verifies readiness.
            }
            // The shared Rust flow waits out the learned access point warm-up.
            checkAttempt(attempt);
            return new JSONObject().put("peripheral_id", id).put("model", model)
                    .put("powered_on_already", alreadyOn).put("ap_ready", ready)
                    .put("credentials", new JSONObject()
                            .put("ssid", ssid).put("passphrase", password));
        } catch (Exception error) {
            // The camera dropped the link before giving its credentials. Rust
            // retries this once, as a transient drop, before asking to pair.
            boolean dropped = link.disconnected && errorName(error).equals("ConnectionLost");
            link.close();
            MAIN.post(() -> { if (wantsScan) startScan(); });
            if (dropped) throw new Failure("Disconnected");
            throw error;
        }
    }

    /**
     * Connect and discover services. A failed connect is tried again, up to
     * BLE_CONNECT_ATTEMPTS in all; a connect that times out after
     * connectTimeoutMs only maxTimeoutRetries times, because it means the
     * camera is asleep or out of range. Every other failure ends setup at
     * once; the shared Rust flow owns the retries across setups.
     */
    private static Session connectWithRetries(BluetoothDevice device, long connectTimeoutMs,
            int maxTimeoutRetries, long attempt) throws Exception {
        int timeoutRetries = 0;
        for (int round = 1; ; round++) {
            checkAttempt(attempt);
            if (session != null) session.close();
            Session link = new Session(device, attempt);
            session = link;
            try {
                link.timeoutMs = connectTimeoutMs;
                link.connect();
                link.timeoutMs = BLE_CONNECT_TIMEOUT_MS;
                link.discover();
                return link;
            } catch (Exception error) {
                link.close();
                checkAttempt(attempt);
                String name = errorName(error);
                boolean timeoutRetry = name.equals("Timeout") && timeoutRetries < maxTimeoutRetries;
                if ((!name.equals("ConnectFailed") && !timeoutRetry) || round == BLE_CONNECT_ATTEMPTS) {
                    // A drop after the link came up is a transient drop or a lost pairing.
                    if (link.disconnected && name.equals("ConnectionLost")) throw new Failure("Disconnected");
                    throw error;
                }
                if (timeoutRetry) timeoutRetries++;
                pause(BLE_CONNECT_RETRY_DELAY_MS, attempt);
            }
        }
    }

    /**
     * Local network access as Android 17 reports it. Below Android 17, or for
     * an app that targets less, INTERNET implies LAN access and nothing is
     * requested. A permission missing from the manifest is a denial no dialog
     * can lift; a request that gets no answer is unknown.
     */
    @SuppressWarnings("deprecation")
    private static String localNetworkAccess(long attempt) throws Exception {
        Activity host = activity;
        if (host == null) return "unknown";
        if (Build.VERSION.SDK_INT < LOCAL_NETWORK_API
                || host.getApplicationInfo().targetSdkVersion < LOCAL_NETWORK_API) return "granted";
        if (granted(LOCAL_NETWORK)) return "granted";
        String[] declared;
        try {
            declared = host.getPackageManager().getPackageInfo(host.getPackageName(),
                    PackageManager.GET_PERMISSIONS).requestedPermissions;
        } catch (PackageManager.NameNotFoundException error) { declared = null; }
        if (declared == null || !Arrays.asList(declared).contains(LOCAL_NETWORK)) return "denied";
        Boolean answer;
        try { answer = requestPermission(LOCAL_NETWORK, attempt, LOCAL_NETWORK_REQUEST_MS); }
        catch (TimeoutException error) { return "unknown"; }
        return answer == null ? "unknown" : answer ? "granted" : "denied";
    }

    /** Ask for one permission: true, false, or null when the dialog closed without an answer. */
    private static Boolean requestPermission(String permission, long attempt, long timeoutMs) throws Exception {
        CompletableFuture<Boolean> answer = new CompletableFuture<>();
        permissionAnswer = answer;
        MAIN.post(() -> {
            Activity host = activity;
            if (host != null && attempt == generation) host.requestPermissions(new String[]{permission}, SINGLE_PERMISSION);
            else answer.complete(null);
        });
        try { return answer.get(timeoutMs, TimeUnit.MILLISECONDS); }
        finally { if (permissionAnswer == answer) permissionAnswer = null; }
    }

    /** This app's details page, where its permissions are granted. */
    private static void openSettings() throws Failure {
        Activity host = activity;
        if (host == null) throw new Failure("Cancelled");
        Intent intent = new Intent(android.provider.Settings.ACTION_APPLICATION_DETAILS_SETTINGS,
                Uri.fromParts("package", host.getPackageName(), null));
        host.runOnUiThread(() -> host.startActivity(intent));
    }

    private static String credential(byte[] bytes) {
        int end = 0;
        while (end < bytes.length && bytes[end] != 0) end++;
        return new String(bytes, 0, end, StandardCharsets.UTF_8).trim();
    }

    private static void checkAttempt(long attempt) throws Failure {
        if (attempt != generation || activity == null) throw new Failure("Cancelled");
    }

    private static void pause(long milliseconds, long attempt) throws Exception {
        long deadline = SystemClock.elapsedRealtime() + milliseconds;
        while (SystemClock.elapsedRealtime() < deadline) {
            checkAttempt(attempt);
            Thread.sleep(Math.min(250, Math.max(1, deadline - SystemClock.elapsedRealtime())));
        }
        checkAttempt(attempt);
    }

    private static final class Failure extends Exception {
        private static final long serialVersionUID = 1L;
        Failure(String message) { super(message); }
    }

    private static final class Result {
        final int status;
        final byte[] data;
        Result(int status, byte[] data) { this.status = status; this.data = data; }
    }

    /** One outstanding GATT operation; callbacks never block the Android binder thread. */
    private static final class Session extends BluetoothGattCallback {
        final BluetoothDevice device;
        /** The attempt that owns this link; a cancel teardown adopts it. */
        volatile long attempt;
        final BlockingQueue<Boolean> apReady = new LinkedBlockingQueue<>();
        volatile BluetoothGatt gatt;
        volatile CompletableFuture<Result> pending;
        volatile UUID expected;
        volatile String operation;
        volatile long timeoutMs = BLE_CONNECT_TIMEOUT_MS;
        volatile boolean closed;
        volatile boolean connected;
        volatile boolean disconnected;
        Session(BluetoothDevice device, long attempt) { this.device = device; this.attempt = attempt; }
        void begin(String kind, UUID uuid) throws Failure {
            checkAttempt(attempt);
            if (closed) throw new Failure("ConnectionLost");
            operation = kind; expected = uuid; pending = new CompletableFuture<>();
        }
        Result await() throws Exception {
            try { return pending.get(timeoutMs, TimeUnit.MILLISECONDS); }
            catch (TimeoutException error) {
                // Android may hold the encrypted read while its pairing dialog is open.
                if (operation.equals("read") && device.getBondState() == BluetoothDevice.BOND_BONDING) {
                    try { return pending.get(Math.max(1, PAIRING_WAIT_MS - timeoutMs), TimeUnit.MILLISECONDS); }
                    catch (TimeoutException pairingTimeout) { close(); throw new Failure("Timeout"); }
                }
                close(); throw new Failure("Timeout");
            }
        }
        void deliver(String kind, UUID uuid, int status, byte[] bytes) {
            CompletableFuture<Result> waiting = pending;
            if (!closed && waiting != null && kind.equals(operation) && Objects.equals(uuid, expected))
                waiting.complete(new Result(status, bytes));
        }
        void success(Result result) throws Failure {
            if (result.status != BluetoothGatt.GATT_SUCCESS) throw new Failure("ConnectionLost");
        }
        void connect() throws Exception {
            begin("connect", null);
            gatt = device.connectGatt(activity, false, this, BluetoothDevice.TRANSPORT_LE);
            if (gatt == null) throw new Failure("ConnectFailed");
            if (await().status != BluetoothGatt.GATT_SUCCESS) throw new Failure("ConnectFailed");
        }
        void discover() throws Exception {
            begin("discover", null);
            if (!gatt.discoverServices()) throw new Failure("ConnectionLost");
            success(await());
        }
        BluetoothGattCharacteristic characteristic(UUID uuid) throws Failure {
            if (gatt != null) for (BluetoothGattService service : gatt.getServices()) {
                BluetoothGattCharacteristic value = service.getCharacteristic(uuid);
                if (value != null) return value;
            }
            throw new Failure("NotPaired");
        }
        byte[] read(UUID uuid) throws Exception {
            BluetoothGattCharacteristic value = characteristic(uuid);
            for (int retry = 0; retry < 2; retry++) {
                begin("read", uuid);
                if (!gatt.readCharacteristic(value)) throw new Failure("ConnectionLost");
                Result result = await();
                if (result.status == BluetoothGatt.GATT_SUCCESS) return result.data;
                if (result.status != BluetoothGatt.GATT_INSUFFICIENT_AUTHENTICATION
                        && result.status != BluetoothGatt.GATT_INSUFFICIENT_ENCRYPTION
                        && result.status != 137) throw new Failure("ConnectionLost");
                // The camera refused the read for want of pairing. Wait for one
                // system pairing and read once more; any other outcome is an
                // authentication failure the shared flow retries.
                if (retry != 0) throw new Failure("Authentication");
                int bond = device.getBondState();
                // A bond the camera no longer honours cannot be repaired from here.
                if (bond == BluetoothDevice.BOND_BONDED
                        || (bond == BluetoothDevice.BOND_NONE && !device.createBond()))
                    throw new Failure("Authentication");
                long deadline = SystemClock.elapsedRealtime() + PAIRING_WAIT_MS;
                long bondingGrace = SystemClock.elapsedRealtime() + 2000;
                do {
                    checkAttempt(attempt);
                    Thread.sleep(250);
                    if (device.getBondState() == BluetoothDevice.BOND_BONDED) break;
                    if (device.getBondState() == BluetoothDevice.BOND_NONE
                            && SystemClock.elapsedRealtime() >= bondingGrace) throw new Failure("Authentication");
                } while (SystemClock.elapsedRealtime() < deadline);
                if (device.getBondState() != BluetoothDevice.BOND_BONDED) throw new Failure("Timeout");
            }
            throw new Failure("Authentication");
        }
        @SuppressWarnings("deprecation")
        void write(UUID uuid, byte[] bytes) throws Exception {
            BluetoothGattCharacteristic value = characteristic(uuid);
            begin("write", uuid);
            boolean accepted;
            if (Build.VERSION.SDK_INT >= 33) accepted = gatt.writeCharacteristic(value, bytes,
                    BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT) == BluetoothStatusCodes.SUCCESS;
            else {
                value.setWriteType(BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT);
                value.setValue(bytes);
                accepted = gatt.writeCharacteristic(value);
            }
            if (!accepted) throw new Failure("ConnectionLost");
            success(await());
        }
        @SuppressWarnings("deprecation")
        void subscribe() throws Exception {
            BluetoothGattCharacteristic value = characteristic(NETWORK);
            if (!gatt.setCharacteristicNotification(value, true)) throw new Failure("ConnectionLost");
            BluetoothGattDescriptor descriptor = value.getDescriptor(CCCD);
            if (descriptor == null) throw new Failure("ConnectionLost");
            begin("descriptor", CCCD);
            byte[] bytes = BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE;
            boolean accepted;
            if (Build.VERSION.SDK_INT >= 33) accepted = gatt.writeDescriptor(descriptor, bytes) == BluetoothStatusCodes.SUCCESS;
            else { descriptor.setValue(bytes); accepted = gatt.writeDescriptor(descriptor); }
            if (!accepted) throw new Failure("ConnectionLost");
            success(await());
        }
        /** Fail the operation in flight but keep the link, for a teardown that still needs it. */
        void abort() {
            CompletableFuture<Result> waiting = pending;
            if (waiting != null) waiting.completeExceptionally(new Failure("Cancelled"));
        }
        void close() {
            closed = true;
            CompletableFuture<Result> waiting = pending;
            if (waiting != null) waiting.completeExceptionally(new Failure("ConnectionLost"));
            BluetoothGatt old = gatt;
            gatt = null;
            if (old != null) {
                try { old.disconnect(); old.close(); } catch (SecurityException ignored) { }
            }
        }
        @Override public void onConnectionStateChange(BluetoothGatt source, int status, int state) {
            if (closed) return;
            if (state == BluetoothProfile.STATE_CONNECTED) {
                if (status == BluetoothGatt.GATT_SUCCESS) connected = true;
                disconnected = false;
                deliver("connect", null, status, null);
            } else if (state == BluetoothProfile.STATE_DISCONNECTED) {
                disconnected = true;
                CompletableFuture<Result> waiting = pending;
                // A connect the stack ends (status 133 and the like) failed; any
                // later drop loses the link.
                if (waiting != null) waiting.completeExceptionally(new Failure(
                        "connect".equals(operation) ? "ConnectFailed" : "ConnectionLost"));
                event("disconnected", JSONObject.quote(device.getAddress()));
            }
        }
        @Override public void onServicesDiscovered(BluetoothGatt source, int status) { deliver("discover", null, status, null); }
        @Override public void onCharacteristicRead(BluetoothGatt source, BluetoothGattCharacteristic value, byte[] bytes, int status) {
            deliver("read", value.getUuid(), status, bytes.clone());
        }
        @Override @SuppressWarnings("deprecation")
        public void onCharacteristicRead(BluetoothGatt source, BluetoothGattCharacteristic value, int status) {
            if (Build.VERSION.SDK_INT < 33) {
                byte[] bytes = value.getValue();
                deliver("read", value.getUuid(), status, bytes == null ? new byte[0] : bytes.clone());
            }
        }
        @Override public void onCharacteristicWrite(BluetoothGatt source, BluetoothGattCharacteristic value, int status) {
            deliver("write", value.getUuid(), status, null);
        }
        @Override public void onDescriptorWrite(BluetoothGatt source, BluetoothGattDescriptor value, int status) {
            deliver("descriptor", value.getUuid(), status, null);
        }
        @Override public void onCharacteristicChanged(BluetoothGatt source, BluetoothGattCharacteristic value, byte[] bytes) {
            if (!closed && value.getUuid().equals(NETWORK) && bytes != null && bytes.length > 0 && bytes[0] == 1) apReady.offer(true);
        }
        @Override @SuppressWarnings("deprecation")
        public void onCharacteristicChanged(BluetoothGatt source, BluetoothGattCharacteristic value) {
            if (Build.VERSION.SDK_INT < 33) onCharacteristicChanged(source, value, value.getValue());
        }
    }

    /**
     * Join the camera access point through a network request. Every failure
     * the camera may still disprove over HTTP is a soft "WifiJoinFailed" or
     * "WifiJoinTimeout": Android reports a declined system dialog and a camera
     * network it cannot find through the same onUnavailable. Camera traffic
     * is routed to the joined network per request (see http), so the process
     * stays on the default network and internet downloads keep working.
     */
    private static void join(JSONObject args, long attempt) throws Exception {
        if (Build.VERSION.SDK_INT < 29) throw new Failure("WifiJoinFailed");
        String permission = Build.VERSION.SDK_INT >= 33 ? Manifest.permission.NEARBY_WIFI_DEVICES
                : Manifest.permission.ACCESS_FINE_LOCATION;
        if (!granted(permission) && !Boolean.TRUE.equals(requestPermission(permission, attempt, 90_000))) {
            checkAttempt(attempt);
            throw new Failure("WifiJoinDenied");
        }
        checkAttempt(attempt);
        ConnectivityManager manager = activity.getSystemService(ConnectivityManager.class);
        if (args.optBoolean("force")) reassociate(manager, attempt);
        String ssid = args.getString("ssid");
        CompletableFuture<Network> ready = new CompletableFuture<>();
        joining = ready;
        WifiNetworkSpecifier specifier = new WifiNetworkSpecifier.Builder().setSsid(ssid)
                .setWpa2Passphrase(args.getString("passphrase")).build();
        NetworkRequest request = new NetworkRequest.Builder().addTransportType(NetworkCapabilities.TRANSPORT_WIFI)
                .removeCapability(NetworkCapabilities.NET_CAPABILITY_INTERNET).setNetworkSpecifier(specifier).build();
        ConnectivityManager.NetworkCallback callback = new ConnectivityManager.NetworkCallback() {
            @Override public void onAvailable(Network network) {
                synchronized (CameraBridge.class) {
                    if (attempt == generation && networkCallback == this) {
                        cameraNetwork = network; ready.complete(network);
                    }
                }
            }
            // The user declined the system dialog, or the camera network was not found.
            @Override public void onUnavailable() { fail(null); }
            // Losing the camera network ends the link, during the join or after it.
            @Override public void onLost(Network network) { fail(network); }
            private void fail(Network lost) {
                boolean established;
                synchronized (CameraBridge.class) {
                    if (networkCallback != this) return;
                    established = lost != null && lost.equals(cameraNetwork);
                    releaseNetwork();
                }
                ready.completeExceptionally(new Failure("WifiJoinFailed"));
                // Tell Rust at once: camera requests have no route from now on.
                if (established) event("networkLost", "true");
            }
        };
        synchronized (CameraBridge.class) {
            releaseNetwork();
            networkCallback = callback;
            manager.requestNetwork(request, callback);
        }
        try { awaitJoin(ready, attempt); checkAttempt(attempt); }
        catch (Exception error) {
            synchronized (CameraBridge.class) {
                if (networkCallback == callback) releaseNetwork();
            }
            if (error instanceof TimeoutException) throw new Failure("WifiJoinTimeout");
            throw error;
        } finally { if (joining == ready) joining = null; }
    }

    /**
     * Wait for the camera network for JOIN_TIMEOUT_MS of foreground time.
     * While the system dialog covers the app, the wait does not count down.
     */
    private static void awaitJoin(CompletableFuture<Network> ready, long attempt) throws Exception {
        long counted = 0;
        long last = SystemClock.elapsedRealtime();
        while (true) {
            try { ready.get(JOIN_POLL_MS, TimeUnit.MILLISECONDS); return; }
            catch (TimeoutException ignored) { /* Still joining. */ }
            checkAttempt(attempt);
            long now = SystemClock.elapsedRealtime();
            if (resumed) counted += now - last;
            last = now;
            if (counted >= JOIN_TIMEOUT_MS) throw new TimeoutException();
        }
    }

    /**
     * Forced reassociation: drop the camera network, wait until the system
     * has let it go, then let the caller join again. A network that does not
     * go away in time is a soft failure; joining over it would reuse the
     * route that never reached the camera.
     */
    private static void reassociate(ConnectivityManager manager, long attempt) throws Exception {
        Network previous;
        synchronized (CameraBridge.class) {
            previous = cameraNetwork;
            releaseNetwork();
        }
        long deadline = SystemClock.elapsedRealtime() + REASSOCIATE_SETTLE_TIMEOUT_MS;
        pause(REASSOCIATE_SETTLE_MIN_MS, attempt);
        while (previous != null && manager.getNetworkCapabilities(previous) != null) {
            if (SystemClock.elapsedRealtime() >= deadline) throw new Failure("WifiJoinFailed");
            pause(REASSOCIATE_SETTLE_POLL_MS, attempt);
        }
    }

    private static synchronized void releaseNetwork() {
        ConnectivityManager.NetworkCallback callback = networkCallback;
        networkCallback = null;
        cameraNetwork = null;
        if (callback != null && activity != null) {
            try { activity.getSystemService(ConnectivityManager.class).unregisterNetworkCallback(callback); }
            catch (IllegalArgumentException ignored) { /* The system already dropped the request. */ }
        }
    }

    /**
     * The address camera traffic leaves from: the camera network's own
     * link properties while the app holds it, else the active network's.
     * Only a Wi-Fi network counts; the interface list cannot tell which
     * Wi-Fi interface carries the camera when the phone keeps two.
     */
    private static JSONObject networkSnapshot() throws JSONException {
        JSONObject result = new JSONObject().put("wifi", false).put("address", JSONObject.NULL);
        Activity host = activity;
        if (host == null) return result;
        ConnectivityManager manager = host.getSystemService(ConnectivityManager.class);
        if (manager == null) return result;
        Network network = cameraNetwork;
        if (network == null) network = manager.getActiveNetwork();
        if (network == null) return result;
        NetworkCapabilities capabilities = manager.getNetworkCapabilities(network);
        if (capabilities == null || !capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)) return result;
        result.put("wifi", true);
        LinkProperties properties = manager.getLinkProperties(network);
        if (properties != null) for (LinkAddress link : properties.getLinkAddresses()) {
            if (link.getAddress() instanceof Inet4Address) {
                result.put("address", link.getAddress().getHostAddress());
                break;
            }
        }
        return result;
    }

    /** Routes a socket that Rust opened over the camera network, as {@link #http} does. */
    public static void bindSocket(int fd) throws IOException {
        Network network = cameraNetwork;
        if (network == null) return;
        // The duplicate refers to the same socket; closing it leaves Rust's descriptor open.
        try (ParcelFileDescriptor socket = ParcelFileDescriptor.fromFd(fd)) {
            network.bindSocket(socket.getFileDescriptor());
        }
    }

    private static JSONObject http(long id, JSONObject args) throws Exception {
        URL url = new URL(args.getString("url"));
        if (!url.getProtocol().equals("http") && !url.getProtocol().equals("https")) throw new IOException("Unsupported protocol");
        Network network = cameraNetwork;
        HttpURLConnection connection = (HttpURLConnection) (network == null ? url.openConnection() : network.openConnection(url));
        requests.put(id, connection);
        try {
            if (cancelledRequests.contains(id)) throw new Failure("Cancelled");
            int timeout = Math.max(1, Math.min(180000, args.optInt("timeout_ms", 30000)));
            connection.setConnectTimeout(Math.min(3000, timeout));
            connection.setReadTimeout(timeout);
            connection.setInstanceFollowRedirects(false);
            String method = args.getString("method");
            connection.setRequestMethod(method);
            JSONObject headers = args.optJSONObject("headers");
            if (headers != null) for (Iterator<String> keys = headers.keys(); keys.hasNext();) {
                String key = keys.next(); connection.setRequestProperty(key, headers.getString(key));
            }
            if (args.has("body") && !args.isNull("body")
                    && (!args.getString("body").isEmpty() || method.equals("PUT") || method.equals("POST"))) {
                byte[] body = Base64.decode(args.getString("body"), Base64.DEFAULT);
                connection.setDoOutput(true);
                connection.setFixedLengthStreamingMode(body.length);
                try (OutputStream output = connection.getOutputStream()) { output.write(body); }
            }
            int status = connection.getResponseCode();
            JSONObject responseHeaders = new JSONObject();
            for (Map.Entry<String, List<String>> header : connection.getHeaderFields().entrySet())
                if (header.getKey() != null) responseHeaders.put(header.getKey(), String.join(", ", header.getValue()));
            InputStream input = status >= 400 ? connection.getErrorStream() : connection.getInputStream();
            ByteArrayOutputStream bytes = new ByteArrayOutputStream();
            if (input != null) try (InputStream stream = input) {
                byte[] buffer = new byte[16384];
                for (int count; (count = stream.read(buffer)) != -1;) {
                    if (bytes.size() + count > 16 * 1024 * 1024) throw new IOException("Camera response too large");
                    bytes.write(buffer, 0, count);
                }
            }
            return new JSONObject().put("status", status).put("headers", responseHeaders)
                    .put("body", Base64.encodeToString(bytes.toByteArray(), Base64.NO_WRAP));
        } finally { requests.remove(id); connection.disconnect(); }
    }

    /** The attempt succeeded: scanning resumes if the app still wants results. */
    private static void finish() {
        connecting = false;
        if (wantsScan) startScan();
    }

    /**
     * The camera session ended: drop the Bluetooth link the connect kept, so
     * the camera advertises again. A camera on an open link does not
     * advertise, and the nearby watch would never see it power on. The
     * access point and the camera network stay as they are.
     */
    private static void releaseLink() {
        if (connecting) return;
        Session old = session;
        session = null;
        if (old != null) old.close();
        if (wantsScan) startScan();
    }

    /**
     * End the current attempt. With `teardown`, a camera the attempt reached
     * over Bluetooth is told to close its access point before the link drops.
     */
    private static void cancel(boolean teardown) {
        generation++;
        connecting = false;
        Session old = session;
        session = null;
        if (old != null) {
            if (teardown && old.connected) {
                // Setup, if still running, fails at its next step and closes the
                // link; the teardown then takes it back.
                old.abort();
                long owner = generation;
                RADIO.execute(() -> stopAccessPoint(old, owner));
            } else old.close();
        }
        CompletableFuture<Network> pendingJoin = joining;
        if (pendingJoin != null) pendingJoin.completeExceptionally(new Failure("Cancelled"));
        CompletableFuture<Boolean> pendingPermission = permissionAnswer;
        if (pendingPermission != null) pendingPermission.completeExceptionally(new Failure("Cancelled"));
        releaseNetwork();
        if (wantsScan) startScan();
    }

    /**
     * Write NETWORK_TYPE 0 so the camera closes its access point, then drop
     * the link. Best effort and bounded: a dropped link is taken back within
     * TEARDOWN_RECONNECT_TIMEOUT_MS and the write gets TEARDOWN_COMMAND_TIMEOUT_MS.
     * Runs on the radio thread, so the next attempt's setup waits for it.
     */
    private static void stopAccessPoint(Session old, long owner) {
        Session link = old;
        try {
            // A later cancel or shutdown owns the camera now.
            if (owner != generation) return;
            if (link.closed || link.disconnected || link.gatt == null) {
                link.close();
                link = new Session(old.device, owner);
                long deadline = SystemClock.elapsedRealtime() + TEARDOWN_RECONNECT_TIMEOUT_MS;
                link.timeoutMs = TEARDOWN_RECONNECT_TIMEOUT_MS;
                link.connect();
                link.timeoutMs = Math.max(1, deadline - SystemClock.elapsedRealtime());
                link.discover();
            } else {
                link.attempt = owner;
            }
            link.timeoutMs = TEARDOWN_COMMAND_TIMEOUT_MS;
            link.write(NETWORK, new byte[]{0});
        } catch (Exception ignored) {
            // Best effort: the attempt is already over.
        } finally {
            link.close();
            old.close();
        }
    }

    /**
     * The user disconnects or turns the camera off: write NETWORK_TYPE 0 so it
     * closes its access point, or CAMERA_POWER 0 so it turns off. Uses the
     * current link when it is up, otherwise connects once, bounded like a
     * teardown. A power off may drop the link before the write is answered.
     */
    private static void endSession(JSONObject args, long attempt) throws Exception {
        if (!radioPermissions()) throw new Failure("BluetoothUnavailable");
        BluetoothAdapter adapter = adapter();
        if (adapter == null || !adapter.isEnabled()) throw new Failure("BluetoothUnavailable");
        BluetoothDevice device;
        try { device = adapter.getRemoteDevice(args.getString("id")); }
        catch (IllegalArgumentException error) { throw new Failure("NotNearby"); }
        boolean powerOff = args.optBoolean("power_off");
        Session current = session;
        boolean reuse = current != null && !current.closed && !current.disconnected && current.gatt != null
                && current.device.getAddress().equalsIgnoreCase(device.getAddress());
        Session link = reuse ? current : new Session(device, attempt);
        try {
            if (!reuse) {
                long deadline = SystemClock.elapsedRealtime() + TEARDOWN_RECONNECT_TIMEOUT_MS;
                link.timeoutMs = TEARDOWN_RECONNECT_TIMEOUT_MS;
                link.connect();
                link.timeoutMs = Math.max(1, deadline - SystemClock.elapsedRealtime());
                link.discover();
            }
            link.timeoutMs = TEARDOWN_COMMAND_TIMEOUT_MS;
            link.write(powerOff ? POWER : NETWORK, new byte[]{0});
        } finally {
            if (!reuse) link.close();
        }
    }

    public static void shutdown() {
        if (receiverContext != null) {
            receiverContext.unregisterReceiver(ADAPTER_STATE);
            receiverContext = null;
        }
        wantsScan = false;
        requestingPermissions = false;
        stopScan();
        cancel(false);
        synchronized (activeRequests) { cancelledRequests.addAll(activeRequests); }
        for (HttpURLConnection request : requests.values()) request.disconnect();
        activity = null;
    }
}

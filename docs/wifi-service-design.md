# Wi-Fi service v1 design

Status: revised 2026-11-19. Film Looks, recipes, and software update are implemented as this document describes. Module settings are a proposal. The owner replaced the first Look design, which loaded Look files while the camera ran, with the simpler rule of section 8: a choice applies at once, and a file applies at the next start.

GRKit is developing its first public Wi-Fi protocol. Replace the current development endpoints and clients together. Do not add legacy adapters or a v2 namespace.

## 1. Goals and boundaries

The phone manages Film Looks, recipes, and module settings during one Wi-Fi connection. A change of a setting, such as a Look that is on or off, applies at once. A change of a Look file applies when the camera starts again. Software updates require camera confirmation and a restart.

The service exposes business resources over HTTP. A WebSocket tells the phone that a resource changed. There are no public generic upload or operation resources.

| Phone application | Camera service and runtime |
| --- | --- |
| Download the official catalog and release index | Read actual card contents and runtime state |
| Store a local library | Validate and persist camera changes |
| Convert LUT files to GRF | Load validated GRF effects at the start |
| Render previews and report fitting limits | Coordinate changes with capture and menu activity |
| Select files and share content | Export looks and recipes |
| Assemble signed software packages | Verify signatures and install |

BLE discovery, pairing, wake, and Wi-Fi association remain native phone functions. The HTTP service does not replace them. Remote shutter control, photo transfer, and live view are outside this design.

First delivery covers the protocol foundation, Film Looks, and LUT import. Recipes and module settings use the same contracts in later deliveries. Unimplemented capabilities remain unavailable.

## 2. Existing constraints and evidence

The server answers one request on each connection and admits a bounded number of connections.

The installer builds effect records, menu labels, and icons at startup. Repeating the full installer during capture is not a valid hot-loading implementation.

Existing effects can change their enabled state and order at runtime. An effect that has no loaded record currently needs another start. New and removed records need additional runtime support.

Current limits differ: the runtime loads at most 40 effects and lists at most 64. Effect names contain 1 to 16 printable ASCII characters. Capabilities report these limits separately.

The actual effect library is `GRKIT/EFFECTS`. `SCRIPT/GRKIT/EFFECTS` is the package inbox. `PACKAGE.TXT` describes package selections, not the complete live library.

The camera filesystem does not support ordinary Linux replacement semantics in all cases. Rename over an existing file can fail. On GR IV, Linux deletion can make later new filenames invisible until the next power-on.

Linux also caches negative lookups and can miss files created later by the RTOS. The current service reads directory entries to detect renamed journals. Do not assume desktop filesystem transactions or coherent cross-OS caches.

The service writes Look files through Linux. It creates and overwrites files, which work on each card format. It deletes a file only after it has looked up each existing file of the folder, and it then looks up one unused name, so the failed lookup that can follow a delete on GR IV harms no later request. The service does not rename over a file.

The camera reads its Look files only when it starts. The settings of the camera stay in RAM and in the state file, which the resident loader saves. The service never writes the state file.

Recipe records currently use 15-bit effect codes. INSTALL resolves record-code collisions by incrementing codes in catalog order. Recipes instead find effects by the original ID hash. Adding or removing an effect can therefore alter persisted references.

Admission must reject a new ID whose effect code collides with another library ID or a reserved native code. Return 409 `effect_code_conflict`. The phone can generate another ID before compiling an imported LUT.

Existing ambiguous libraries remain readable, but affected mutations are blocked. Do not silently change IDs or reassign codes. Inventory reports collisions before the user chooses how to resolve them.

Recipes already have generated stable numeric IDs. Reuse those IDs. An ID-less recipe is readable in inventory but cannot be mutated by index. Runtime ID assignment and its failure case still need a defined recovery path.

Implementation references:

- [`crates/grkit-serve/src/lib.rs`](../crates/grkit-serve/src/lib.rs)
- [`crates/grkit-serve/src/main.rs`](../crates/grkit-serve/src/main.rs)
- [`runtime/installer.c`](../runtime/installer.c)
- [`runtime/effects.h`](../runtime/effects.h)
- [`runtime/install_library.h`](../runtime/install_library.h)
- [`crates/grkit-core/src/card.rs`](../crates/grkit-core/src/card.rs)
- [`crates/grkit-core/src/recipes/model.rs`](../crates/grkit-core/src/recipes/model.rs)
- [`crates/grkit-wasm/src/lib.rs`](../crates/grkit-wasm/src/lib.rs)

GR native API research informed naming: `/props`, `/params/camera`, `/changes`, and action suffixes such as `/cancel`. These conventions do not require copying native form encoding or errors inside HTTP 200 responses.

## 3. Protocol conventions

The base URL remains `http://<camera-address>:8080/v1`. This is a separate service from Ricoh's port 80 API. The event URL is `ws://<camera-address>:8080/v1/changes`.

JSON uses camelCase. Binary responses declare their media type. JSON requests require `application/json`, except binary content routes. Unsupported media types return 415.

Use actual HTTP status codes. Errors use `application/problem+json`. JSON responses use `Cache-Control: no-store`. The service sends no ETags.

### Identity

| Field | Meaning |
| --- | --- |
| `deviceId` | Stable identity for the camera, from a verified device identifier |
| `bootId` | New opaque value for each service lifetime, including a service restart |
| `cardId` | Opaque identity for the currently managed card |
| `requestId` | The ID that the service gives a software request that it writes to the card |

Do not use names, menu positions, filenames, or runtime addresses as public identities. Look IDs map to the stable IDs in validated GRF data. Recipe IDs use the existing generated numeric identity, serialized as a string.

The card identity is a random marker file that the service creates once on the card. The service does not accept mutations while it cannot read or create the marker. Formatting the card removes the marker, so the card then has a new identity. A copy of the whole card keeps the marker: the two cards have one identity. Removing the card powers the camera off, so the card cannot change within one `bootId`.

### Mutation admission

One phone uses the service, and its requests come one after another. The only other writer is the person at the camera. The protocol therefore has no optimistic-concurrency preconditions and no request records in memory.

Each mutation carries `X-GRKit-Boot-Id`, the `bootId` that the phone last read. A different or missing value returns 409 `context_changed`: the camera started again, so the phone reads the state again. The card and the camera body cannot change within one `bootId`, so a mutation names neither.

A mutation ends within its HTTP response. After an unknown result, the phone reads the resource again and decides from its state:

- On, off, and the order of the Looks are absolute values. A repeated request sets the same value.
- A Look content PUT replaces the file. A repeated request writes the same file.
- A DELETE of a Look that is gone returns 404.
- An update that already waits on the card makes the repeated request return 409 `software_pending`. `GET /update` then has the request that waits.

Look bodies are only 8192 or 8704 bytes in the current format. Receive each bounded body in RAM and validate it before the write. Do not create a separate persisted upload stage for GRF.

Use a five-second idle read timeout, matching the current server, and a bounded total transfer deadline. The exact software-upload deadline must be measured on physical Wi-Fi. Reject unsupported lengths before allocation.

An interrupted body never changes live resources. Discard it and let the phone resend the complete body.

Software ZIP uploads still need bounded staging and signature validation. Their staged data and physical-confirmation journal survive disconnects. A concurrent duplicate upload must not start another writer. The service checks what needs no body (the `bootId`, a request that already waits) before it receives the package.

The software request that waits is on the card with its journal. The service of the next start reads it there. No other request has a record.

Use a separate configuration representation for Look PATCH. Order and usage information do not belong in that representation. Runtime status and dependencies belong in `/looks/{id}/info` and the Looks list.

## 4. Read model

### Global endpoints

| Method | Path | Response |
| --- | --- | --- |
| GET | `/ping` | Service reachability and `bootId` |
| GET | `/props` | Device, software summary, capabilities, limits, storage, runtime state, and active request summaries |
| GET | `/debug/revisions` | Component versions, JSON |
| GET | `/debug/diagnostics` | Bounded diagnostic report, without Wi-Fi credentials |

`/ping` can succeed while the card or runtime is unavailable. `/props` identifies that condition. It never represents an unreadable library as an empty library.

Example `/props` response, with illustrative values:

```json
{
  "deviceId": "camera-opaque-id",
  "bootId": "boot-opaque-id",
  "cardId": "card-opaque-id",
  "revision": "sha256:configuration-digest",
  "camera": {"model": "gr3", "firmwareVersion": "2.21"},
  "grkit": {"version": "1.7.0"},
  "storage": {"state": "ready", "writable": true, "freeBytes": 1073741824},
  "runtime": {"state": "ready", "busyReason": null},
  "writeAccess": {"allowed": true, "reason": null},
  "capabilities": {
    "looks": {"read": true, "write": true, "appliesAtRestart": ["content", "name", "delete", "covers"]},
    "recipes": {"read": true, "write": true},
    "moduleSettings": {"read": false, "write": false}
  },
  "limits": {
    "loadedLooks": 40,
    "listedLooks": 64,
    "lookNameAsciiCharacters": 16
  },
  "activeRequests": []
}
```

`appliesAtRestart` names the Look changes that wait for the next start. Capability checks include camera family, firmware, loaded modules, and format support.

Free-space values are advisory. The service checks space again before staging. Limits also report maximum body sizes, supported GRF features, and recipe format versions.

### Film Looks

| Method | Path | Purpose |
| --- | --- | --- |
| GET | `/looks` | Complete list and capacity counts |
| GET | `/looks/{id}` | Writable configuration |
| GET | `/looks/{id}/info` | Runtime state, source information, and dependency references |
| GET | `/looks/{id}/content` | Exact GRF bytes, with their SHA-256 in `X-GRKit-Content-SHA256` |
| GET | `/params/looks/order` | Ordered stable IDs |
| PUT | `/looks/{id}/content` | Create or replace a Look from a raw GRF body, with an optional cover |
| PATCH | `/looks/{id}` | Change name or enabled state |
| DELETE | `/looks/{id}` | Remove a Look |
| PUT | `/params/looks/order` | Replace the order with a complete list of IDs |
| GET | `/looks/covers/scheme` | The cover scheme of the built-in Looks |
| PUT | `/looks/covers/scheme` | Give the built-in Looks the covers of another scheme |
| POST | `/looks/covers/reset` | Put the individual covers of one group back to the scheme |

The first version returns the whole bounded library. Pagination is unnecessary at current limits.

Every valid GRF ID remains addressable, including `order` and `requests`. Collection ordering lives under `/params/looks/order`. Recipe IDs are numeric, so recipe collection routes cannot collide with them. Give wildcard routing no priority over a longer, explicit route.

Example configuration from `GET /looks/{id}`:

```json
{
  "id": "lut-example",
  "name": "Example",
  "contentHash": "sha256:...",
  "enabled": true,
  "cover": {"template": "box", "band": true, "brand": "GRKit", "name": "EXAMPLE"}
}
```

`cover` is the cover of the Look, or `null` (see Look covers). The Looks list and the answer of each Look mutation carry it too. The Looks list also has `officialCovers`, the color scheme of the built-in Looks: `gray`, `colorFilmBox`, or `filmBox` (see Look covers). A built-in Look with a base cover also has `defaultCover`: the cover that the scheme gives it. For other Looks it is `null`. The phone sends this cover to put the Look back on the scheme.

`GET /looks/{id}/info` returns loaded state, selection, origin, and dependency references. `/params/looks/order` owns position. The list combines these views.

Enabled permits use. Selected identifies the active Look. Current Look choices have an Off bit and position, but no independent hidden bit. Do not add `hidden` to the v1 Look contract. Recipes retain their existing separate hidden state.

A filename does not prove official origin. The signed release statement of the installed package names each official Look with the SHA-256 of its file and its body hash: the SHA-256 of the file with a fixed name and a blank icon. A Look is `official` when its file or its body matches, so an official Look with another name or cover stays official. An official ID with other colors is `unknown`. A statement without body hashes matches only the file.

Content PUT requires `Content-Length`. The GRF ID must match the path ID. It returns 201 for a new Look and 200 for new content of an existing Look. The server validates the complete effect, features, size, and camera compatibility before application.

A new Look is enabled and appended to the order. It does not replace the active Look. A custom Look replacement preserves identity, position, and enabled state.

PATCH accepts only documented fields. Omitted fields stay unchanged. Unknown fields and invalid values return 422. Renaming a custom or official Look updates the compiled GRF and preserves its ID. A rename also changes the content hash.

An official Look changes its name and its cover as a custom Look does. Its colors cannot change. A content PUT with other colors returns 409 `official_look_read_only`, and so does a rename under a statement without body hashes. Enabling, disabling, ordering, and removal remain available with the normal dependency checks. A Look of `unknown` source cannot change its name or content.

The import library records the name and the cover of each changed official Look (see Look covers).

Official identity must come from trusted catalog or package metadata, not a client-supplied source label. Retain official identity for removed entries so re-adding an ID cannot bypass the editing rule. An unknown or conflicting origin requires resolution before content replacement.

Re-adding a removed official Look accepts only verified official content for that ID: the official file, or the official body with any name and a known cover. Refreshing official content is a distinct catalog/package reconciliation action, not permission for arbitrary same-ID edits.

A same-name Look with a different ID is a separate resource. A repeated ID with different content is a replacement. The phone asks the person before it replaces a Look.

DELETE rejects dependencies with 409 `dependency_in_use`. Dependencies include recipes and dial sets. If dependency scanning is unavailable or ambiguous, deletion fails closed.

Deleting the selected Look requires `?active=off`. Without this explicit policy, return 409 `active_look`. The camera switches to Off when the Look turns off.

Replacing a referenced custom Look keeps the reference but changes its appearance after the next start. The phone uses `/info` to explain this effect before submission.

If informed acknowledgement must bind the exact dependency set, add a dependency-revision precondition before shipping that feature. An unversioned boolean query parameter is not such a precondition.

A library containing duplicate files with the same GRF ID reports `duplicate_id`. The current catalog selects the smallest filename. Block mutations for that ID until the duplicate is resolved. Writing the canonical filename alone cannot guarantee which content loads.

Order PUT accepts `{"ids":["id-a","id-b"]}`. Each library ID occurs exactly once. A missing, duplicate, or unknown ID rejects the whole request.

Capacity validation considers both listed and loaded limits. Do not silently accept a new enabled Look that cannot load. Return required and available capacity in the error.

Multi-file batch import is deferred. A phone can issue separate requests and report each result. Do not claim atomicity across those requests.

### Look covers

A GRF holds only the pixels of its 40 × 40 icon. The cover is the parameters that draw the icon: the layout, the texts, and the colors. The phone shows and edits only covers, for custom and official Looks. It does not send or show icon images.

The parameters travel with the Look on the card:

- The import library, `SCRIPT/GRKIT/IMPORTS.BIN`, keeps the cover of each Look with a copy of its GRF. The site writes the same file. The camera does not read it, and a software update keeps it.
- The signed release statement keeps the original cover of each official Look.

The built-in Looks are the official Looks and the native looks of GR IV that the site adds from the firmware of the user: Cinema Yellow (`gr4-cinema-yellow`) and Cinema Green (`gr4-cinema-green`). The service knows a native look by its ID, and reports its `source` as `native`. A native look changes its name and cover as a custom Look does, and can be deleted. Its import in the library has `"kind": "native"`, which the site writes. The service changes only an existing import of a native look.

A built-in Look has two cover variants of its base cover. The base cover of an official Look is its statement cover. The base cover of a native look is its `cine` cover in the gray colors (`cover::native_cover`, equal to `NATIVE_LOOKS` of the site). The gray cover is the base cover. The film box cover is the base cover with the colors of its film box packaging: Cinema Yellow and Cinema Green get their own colors. The site and the service use the same packaging rule. A variant is never an individual cover. For an official Look, only an individual cover and a custom name go into the library. The import of a native look keeps the cover of its file, as the site does.

The scheme is not stored. The service gets it from the card. It looks at the built-in Looks without an individual cover, in two groups: color Looks and B&W Looks. A B&W Look is an official monochrome GRF. A native look is a color Look. A Look whose film box cover is its gray cover does not count. A group has the film box cover when each of its Looks has it. A group without Looks on the card follows the scheme that the phone last set in this start, else it does not decide. The scheme is:

- `filmBox` when both groups have the film box cover.
- `colorFilmBox` when only the color group has the film box cover, or when the card has only color Looks and they have it. A B&W Look then has the gray cover.
- `gray` in all other cases, such as a mix in the color group, or the film box cover only on B&W Looks.

Another build of the drawing can change some pixels of an icon. So for the scheme an icon counts as the film box cover when fewer of its bytes differ from it than from the gray cover. Such an icon is not an individual cover. Its `cover` is `null`, because the service shows only a cover that draws the icon exactly.

The phone gives the cover of the scheme to each official Look that it adds without an individual cover.

`PUT /looks/covers/scheme` with `{"scheme": "gray" | "colorFilmBox" | "filmBox"}` sets the scheme. The service rewrites the file of each built-in Look without an individual cover whose icon is not the cover of the new scheme. The name and the colors of the Look stay. A Look with an individual cover keeps it. `POST /looks/covers/reset` with `{"group": "color" | "mono"}` rewrites each built-in Look of that group with an individual cover to the cover of the current scheme, and removes the individual cover from the library. The name stays. Both requests follow the other Look mutations: they need `X-GRKit-Boot-Id`, the camera shows the new covers at its next start, and the WebSocket reports `changed: looks`. The answer is `{"officialCovers": scheme, "changed": [ids]}`: the scheme of the card after the change and the IDs of the rewritten files. A request that changes the same scheme again rewrites no file. A failed write returns the problem of the file write, and the Looks before it keep their new files. A request with another body returns 422 `invalid_covers`.

The `cover` of a Look is the first of these that draws the icon of the file on the card:

1. The cover of the library entry of its ID.
2. For an `official` Look, its statement cover, then its film box cover.

Otherwise `cover` is `null`. The service never returns a cover that does not draw the icon. For a Look with `cover: null`, the phone draws the default cover of its name.

A content PUT can carry the cover in `X-GRKit-Cover`, as compact ASCII JSON of at most 1024 bytes. A longer value returns 413 `cover_too_large`. The cover must be valid and draw the icon of the body. Otherwise the request returns 422 `cover_mismatch` and changes nothing. The service writes the GRF, then the library entry. Without the header, the library does not change.

For an existing `official` Look, a PUT needs the official body hash. The name and the icon can differ. The PUT also needs the header, unless a variant draws the icon. Any other PUT for it returns 409 `official_look_read_only`.

Each change of an official Look also writes its library entry, before the GRF. The entry has the written GRF and the details `{"kind":"cover","cover":{...},"name":"..."}`. The details keep other fields of an existing entry. `cover` is absent when the cover is a variant or is not known. `name` is absent when the name is the official one. Without both, the entry goes away. An entry that the library cannot take returns 409 `cover_library_unavailable` and changes nothing. The name is the official one when the GRF, with the icon of the statement cover or with its own icon, has the official file hash.

The service reads the library at most once while it runs, at the first request that needs it, and then keeps it in memory. Its own writes change that copy. Each write replaces the whole file in place and reads it back. A library without entries stays on the card as a file with zero entries.

A library that the service cannot read, because it is damaged or too large, shows none of its covers. The service never writes over it, because the file can hold the imports of a browser. A PUT with a cover then returns 409 `cover_library_unavailable` and writes nothing. A full library answers the same.

A library write that fails after the GRF write returns 507 `cover_not_saved`. The Look keeps its new file. Its `cover` is `null`, unless the library still has a cover that draws the new icon. The phone can send the cover again.

A rename keeps the cover, except the default cover of the old name. That cover names the Look, so the renamed file gets the default cover of the new name, when that fits. The library entry of the ID gets the renamed file. A removed Look loses its library entry. If that library write fails, the removal still succeeds: no Look shows the entry. On, off, and the order do not change the library.

### Recipes

| Method | Path | Purpose |
| --- | --- | --- |
| GET | `/recipes` | List the recipes: settings, on/off, order, names, the current recipe, the undo count |
| POST | `/recipes` | Add a preset of the release index: `{"preset": id}` |
| PATCH | `/recipes/{id}` | Turn on or off (`{"enabled": bool}`), or rename (`{"name": string \| null}`) |
| DELETE | `/recipes/{id}` | Remove a recipe |
| PUT | `/recipes/order` | Replace the order of the recipes that are on: `{"ids": [...]}` |
| POST | `/recipes/{id}/apply` | Apply a recipe to the camera settings |
| POST | `/recipes/undo` | Undo the last apply, with the undo steps of ADJ |

The ID of a recipe is the stable ID that the runtime keeps with the recipe when it moves. The list gives each recipe its semantic settings, its Look ID and name for a recipe on a Look, its readiness, and its automatic name when it has no name. The recipes that are on come first, at most 40. The card keeps 64.

Each change applies at once. The runtime changes the recipes in camera RAM, and the resident loader saves the recipe file after the change, as after a change in MENU. A preset that names a Look that is not on the card returns 409 `recipe_look_missing`; the phone adds the Look first. Other errors: 409 `menu_full`, `card_full`, `nothing_to_undo`, `recipe_unsupported`, `runtime_changed`, and 422 `invalid_name`.

Recipe parameters are read-only. The camera makes recipes with Save Current Settings. Portable recipe JSON and sharing are not in v1.

### Modules and current parameters

| Method | Path | Purpose |
| --- | --- | --- |
| GET | `/modules` | Installed modules and runtime availability |
| GET | `/modules/{id}` | Module metadata and setting definitions |
| GET, PATCH | `/params/modules/{id}` | Read or change module values |
| GET, PATCH | `/params/camera` | Read or change the active Look and its adjustments |

Definitions include key, type, allowed values, range, step, unit, dependencies, read-only reason, and application mode. Values use stable symbolic setting names.

PATCH groups related values into one request. The service validates the final combination. Unsupported or read-only values reject the entire request.

A parameter request succeeds only after runtime acknowledgement and persistence. The implementation can coalesce slider input in the phone. It must not claim persistence before the durable state exists.

Installing executable modules remains part of signed software installation. These endpoints cannot upload code, write arbitrary paths, or execute commands.

## 5. Software update

| Method | Path | Purpose |
| --- | --- | --- |
| GET | `/update` | Software inventory and current or latest update request |
| PUT | `/update` | Receive a raw signed ZIP and prepare an update |
| POST | `/update/cancel` | Cancel the update that waits, when cancellation is still safe |

The cancel request has no body. A cancel request without a waiting update returns 409 `no_software_request`.

The update ZIP requires `Content-Length`. Official signature and target validation are mandatory: the signature also proves that the package arrived complete.

Software inventory includes package membership and content hashes. The phone must preserve custom library content when preparing an update. The release catalog alone cannot reconstruct imported Looks.

A pending software update blocks data mutations. Reads and cancellation remain available.

The existing physical camera confirmation remains required. Confirmation and reboot are separate phases. Journal disappearance alone never proves success.

Recovery checks the installed manifest and expected content identity, not only a version string. Two builds can share a version.

Data management does not edit signed package code. A software update preserves user data.

At the next start, the installer copies each Look of a new package over the file with the same name in the Looks library. An official Look of the package would then lose its custom name and cover. So the service changes each official Look of the package before it stages the files: the Look gets its custom name from the library, and its individual cover from the library, else the cover that the scheme of the card gives the Look. The service checks the signature of the package before this change. The colors of the Look do not change.

There is no rollback over Wi-Fi. To go back to an earlier version, install its package on the card with a card reader.

## 6. Software request lifecycle

Only an update is a request with a lifecycle: it waits for the confirmation on the camera, then for a restart. The service gives the request an ID and writes it to the card with the journal. `GET /update` returns the request that waits or the last one that ended, in `request`:

```json
{
  "requestId": "6b23bb06-b505-4fa6-bdd4-754018494129",
  "domain": "update",
  "status": "running",
  "phase": "awaitingConfirmation",
  "progress": null,
  "cancellable": true,
  "requiredAction": "confirmOnCamera",
  "result": null,
  "error": null
}
```

| Status | Meaning |
| --- | --- |
| `running` | Waiting for the confirmation or the restart, cancelling, or recovering |
| `succeeded` | The installed software has the expected identity |
| `failed` | Ended without a change of the installed software; `error` says why |
| `cancelled` | Cancellation completed before the camera confirmed |

`phase` is `awaitingConfirmation`, `awaitingRestart`, `cancelling`, `recovering`, or `completed`.

PUT `/update` returns 202 with this summary while the request waits, and 200 when it already ended. A cancel request returns 200 with the summary of the cancelled request. A repeated cancel returns the same summary.

Once the camera confirms, finish or recover the transaction. Do not promise cancellation after that boundary. A disconnected client does not cancel work.

The phone never repeats a software request automatically after an unknown result. It reads the resource and continues from the state that it finds.

Recipe changes follow the Looks: the App task sets them at once, with no request record. Parameters and module settings are not implemented; when they are, they follow the same rule.

## 7. WebSocket changes

`GET /changes` upgrades to WebSocket. The service sends events. Clients send no application commands over this socket. Standard WebSocket Ping/Pong handles connection liveness: the service sends a Ping every 15 seconds and closes a socket that does not answer within 30 seconds.

After the upgrade the service sends a `hello` message with the `bootId`. Later messages name a kind of change and carry no data:

```json
{"type": "changed", "changed": "settings"}
```

| `changed` | Meaning | The phone reads |
| --- | --- | --- |
| `looks` | A request of the phone changed the Looks | `/looks` |
| `software` | An update was staged or cancelled | `/update` |
| `settings` | A GRKit setting changed on the camera: MENU, ADJ, a recipe, or the mode dial. This includes a Look that the person turns on or off or moves in the camera menu | The resources that it shows |

### Changes on the camera

The runtime counts the changes of its settings. At each poll of the control mailbox, the App task copies this count into a word of the mailbox, and makes that word readable for Linux only when the count changed. The service compares the word ten times a second while a phone listens. This sends no command to the camera and does not read the card.

The count covers each GRKit setting, so recipes and module settings need no other signal when the service gets them.

### Synchronization

The stream keeps no history and no sequence numbers. The phone opens the socket, waits for `hello`, and then reads what it shows. A change after the `hello` gets an event, so nothing is lost between the two. After a reconnect or a changed `bootId` the phone reads again.

An event is a hint to read. A resource read over HTTP remains authoritative, and each resource works without a socket.

Bound the number of subscribers. A slow subscriber delays only its own events: the service keeps counts, not queues.

The phone holds the stream open while it is connected and in the foreground. A recent frame of the stream shows that the service answers, so the phone does not also poll `/props` for that.

## 8. How Look changes apply

The first design loaded, replaced, and removed Looks while the camera ran. That needed a second copy of the library state, a storage owner in the RTOS, fixed record pools, and a commit protocol with recovery. On the camera it also read the card many times for one change. The owner chose a simpler rule on 2026-11-19.

A Look change is one of two kinds:

| Change | Mechanism | Applies |
| --- | --- | --- |
| On or off, order | The App task sets the choice of the Look, as its checkbox and Order in MENU do | At once |
| New Look, new name, new content or cover | The service writes the Look file, and the cover to the import library | At the next start |
| Removed Look | The Look turns off at once, then the service deletes its file | Hidden at once, gone at the next start |

The camera settings stay the only record of on, off, and order. The card files stay the only record of which Looks exist. No manifest joins them, so a change in the camera menu and a change from the phone cannot disagree.

The service sends a choice through a small mailbox in camera RAM, which the App task polls. The App task answers in one poll and never waits for the card. It changes a choice on the shooting screen, in playback, and on the native pages of MENU: each GRKit row of MENU reads its value when it draws. It does not change a choice while a GRKit page shows (ADJ, a preview, a development, the GRKit settings pages of MENU, or the information page with a GRKit view), because such a page keeps its rows while it shows. Then it answers that the camera is busy, and the service returns 503 `camera_busy` at once. The same rule applies to the recipes.

`GET /looks` joins the Look files with the choices of the camera. A file that the camera does not list has `pending: "add"`. A file whose content changed since the service started has `pending: "update"`. A Look that the camera lists without a file is removed and is not in the list.

A Look that waits for a start has no choice in the camera yet. PATCH of `enabled` for it returns 409 `restart_required`. An order request accepts its ID and sends the camera the order of the other Looks.

A write of a Look file is not a transaction. The service reads the file back after the write. A file that a power loss leaves incomplete fails the check of the installer at the next start, and the camera skips it. A write of the import library is not a transaction either. A library that a power loss damages shows no covers until the site writes it again.

## 9. Package imports and references

The Look files of `GRKIT/EFFECTS` are the library. The installer keeps its rules for a package: a new package copies its Looks into the library, and a copy of the same package does not bring back a Look that was deleted.

A package list can leave out a Look that the person did not choose. The installer hides the library file with that name. When the phone adds that Look, the service gives its file another name, so the camera lists it.

A new package can bring back an official Look that the phone removed, unless the package list leaves that Look out.

Do not edit `PACKAGE.TXT` to represent library changes. A software request compares the hashes of the installed package files, so an edit would make its outcome unknown.

Custom Looks have their own file names, which no package file has. They stay across a software update.

Recipe and dial-set references block the removal of a Look. The camera reports the Look codes that its recipes and mode-dial sets name, and the service matches them with the Look IDs.

When compact runtime codes or reserved native codes collide, reject the new ID during admission. Do not guess which Look a recipe means. Public IDs remain stable across ordering changes and restarts.

## 10. Errors and access policy

Errors follow RFC 9457. `code` is the stable application discriminator. `detail` is diagnostic text and is not parsed by the phone.

```json
{
  "type": "urn:grkit:problem:dependency-in-use",
  "title": "Look has dependent recipes",
  "status": 409,
  "code": "dependency_in_use",
  "requestId": "6b23bb06-b505-4fa6-bdd4-754018494129",
  "recipeIds": ["recipe-a"],
  "retryable": false
}
```

| HTTP status | Examples |
| --- | --- |
| 400 | Invalid JSON or request framing |
| 403 | Phone changes disabled |
| 404 | Unknown resource or request |
| 409 | Busy writer, dependencies, active Look, another start of the camera, a software request that waits |
| 412 | Stale resource or context precondition |
| 413 | Body exceeds declared limits |
| 415 | Unsupported media type |
| 422 | Invalid effect, parameter value, signature, or incompatible content |
| 428 | Missing required resource precondition |
| 503 | Card or runtime temporarily unavailable |
| 507 | Insufficient storage for staging and recovery |

A software request that fails after its HTTP response has the failure in its `error` field. A successful GET of a failed request still returns HTTP 200.

The local camera network is the initial transport trust boundary. The API does not claim that a BLE bond authenticates an HTTP request. There is no account or Internet login.

All writes require a runtime-enforced phone-changes permission. The intended product default is enabled, subject to a final access-policy decision before release. Recheck permission before commit.

Every mutation, including POST actions and cancellation, requires the custom request and context headers. Reject missing headers before processing a body. Reject browser mutations with an `Origin` header. Do not enable wildcard CORS.

WebSocket handshakes do not use CORS preflight. Reject handshakes carrying `Origin`. Native clients must omit it. Validate this behavior with the iOS and Android networking libraries before enabling subscriptions.

Validate `Host` on HTTP and WebSocket handshakes against the local service address and port, using parsed IP literals. Permit the explicit loopback address only in development. Do not accept arbitrary DNS hostnames or trust proxy forwarding headers. This limits DNS rebinding.

These checks reduce browser access. They do not authenticate a hostile native client on the same network. The final local-network trust policy remains a product decision.

Software code requires official signatures. Data requires strict decoding, path containment, content bounds, and reference validation. Uploaded filenames never become arbitrary card paths.

Body lengths, staging bytes, sockets, and WebSocket subscribers have explicit bounds. Disk cleanup never removes active recovery data.

## 11. Delivery and validation

### Delivery order

1. Define schemas, identity, errors, and bounded transport behavior.
2. Verify the mailbox and Look file writes on GR III and GR IV.
3. Implement Look inventory, content transfer, CRUD, and WebSocket notifications.
4. Add the phone library and LUT import flow on iOS and Android.
5. Replace current software update clients with the redesigned v1 contracts.
6. Implement recipes and module settings after the shared control path is verified.

Do not modify product code until the design and prototype scope are approved. This document is not approval to implement all listed features.

### Acceptance matrix

| Area | Required checks |
| --- | --- |
| Basic Looks | Enable and reorder at once. Import, replace, rename, and delete, then start the camera again |
| Official Looks | Accept a new name and a new cover with the official body, reject other colors, record the changes in the library, retain original recipe references |
| Covers | Library and statement covers, film box variants, the three schemes, scheme change and group reset, native looks in the color group, no cover that does not draw the icon, damaged library, failed library write, rename with the default cover, an update that keeps names and covers |
| Rendering | A new Look appears in menus and affects a captured photo after the next start |
| Active Look | Unrelated changes preserve selection and adjustments, explicit selected deletion switches to Off |
| Capacity | Separate listed/loaded limits, exact boundary, no silent omission |
| Dependencies | Recipes and dial sets block deletion, hash/native-code collisions and duplicate files are rejected |
| Persistence | Reboot retains edits, removed official Looks do not return, custom Looks survive an update |
| Concurrency | Camera menu edits race with HTTP writes; each write sets an absolute value |
| Capture safety | Single capture, continuous processing, open menus, and pending parameter saves |
| Networking | Interrupted body, lost response, duplicate request, disconnect during commit, service restart |
| WebSocket | Hello, events for phone requests and for camera settings, reconnect, subscriber limit, no socket available |
| Cancellation | Before receiving finishes, before commit, after commit boundary, repeated cancellation |
| Storage | Card unavailable, full card, card swap, write error, power loss at each recovery phase |
| Updates | Signature rejection, physical confirmation, timeout, reboot verification, incompatible-format rejection before staging |
| Access | Disabled writes, browser POST, WebSocket Origin, DNS rebinding, malformed bodies, arbitrary paths, resource bounds |

Use unit tests for schemas and state transitions. Use firmware emulation for runtime and filesystem failure injection. Test AP continuity and capture behavior on physical cameras.

Emulator transport does not prove physical Wi-Fi stability or power behavior.

### Implementation gates

The following choices require evidence before implementation is considered ready:

- Look file writes and deletes from Linux during capture, on each card format of GR III and GR IV.
- The screens on which the App task changes a Look choice.
- In-memory request retention and total upload deadlines measured on physical Wi-Fi.
- Runtime assignment for ID-less recipes and detection of existing ambiguous effect-code mappings.
- Phone-changes enforcement and the final local-network access policy.


### Approved product decisions

The owner approved these decisions on 2026-11-16:

1. Official Look content remains unchanged. Removal, enablement, ordering, and a new cover remain available.
2. Software packages continue to carry official Looks.

The owner approved these decisions on 2026-11-19, in place of the managed manifest:

3. On, off, and order apply at once through the choices of the camera. A new, renamed, or replaced Look applies at the next start.
4. A removed Look hides in the camera menu at once.
5. An official Look changes its name and its cover as a custom Look does. Its colors stay the official ones.

### Product details still open

- Define how new or revised official catalog content enters an existing library without changing explicit user selections.
- Define the default phone-changes permission and explain pending-update exclusion in the App.

### Review disposition

Claude Code Fable 5.2 reviewed the initial draft. Source checks confirmed filesystem visibility hazards, code collisions, missing capacity, unsupported Look visibility, and route-name collisions.

That revision added global event watermarks, configuration-specific representations, and browser request checks. It removed persisted generic request history and GRF upload staging. Its RTOS storage ownership and its manifest are not part of the design any more (section 8).

The revision of 2026-11-19 removed the event watermarks, the ETags and preconditions, the request records in memory, and the card and device checks of each mutation (sections 3, 6, and 7). One phone uses the service, the card cannot change while the camera is on, and each change sets an absolute value.

The reviewer did not review the later revisions.

## 12. Standards references

- [RFC 9110: HTTP Semantics](https://www.rfc-editor.org/rfc/rfc9110.html): methods, conditional requests, and status codes.
- [RFC 9457: Problem Details](https://www.rfc-editor.org/rfc/rfc9457.html): structured HTTP errors.
- [RFC 6455: WebSocket Protocol](https://www.rfc-editor.org/rfc/rfc6455.html): framing and control messages.

The resource and action names are project conventions. HTTP does not require a particular route spelling. Hot-loading and durability guarantees are GRKit requirements, not guarantees provided by HTTP.

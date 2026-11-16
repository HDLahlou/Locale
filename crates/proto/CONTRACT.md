# Locale interface contract

Generated from `crates/proto` by `cargo run -p proto --bin proto-gen`. Do not edit by hand; a test fails when this file is stale.
The code is the contract (digest §5–§7, §10–§11). Changes go through the orchestrator.

Conventions: timestamps are Unix milliseconds, UTC; IDs are UUIDv4 strings unless stated; JSON uses snake_case; sizes are bytes (binary units); hashes are lowercase hex SHA-256. Base64url is always without padding.

## 1. Transport, join URL, fingerprint (§5.1)

| Item | Value |
|---|---|
| Protocol version | `1` (header `X-Locale-Proto`) |
| Default port | `7420` (next free port if taken) |
| Port sniffing | first byte `0x16` → TLS, otherwise plain HTTP |
| Event ID | `evt_` + 12 lowercase hex digits, e.g. `evt_3f9a0c12b7de` |
| Join token | 16 random bytes, base64url (22 chars) |
| Fingerprint | SHA-256 of the certificate DER (32 bytes), base64url (43 chars) |
| Host code | first 12 hex digits of the fingerprint, uppercase, grouped: `4F2A · 9C1E · 77B0` |

Join URL: `http://<host-ip>:<port>/e/<event-id>#t=<join-token>&fp=<cert-fingerprint>`. IPv6 hosts are bracketed. Example:

```text
http://192.168.1.20:7420/e/evt_3f9a0c12b7de#t=BwcHBwcHBwcHBwcHBwcHBw&fp=TyqcHnewBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8
```

Deep link (DU-11): `locale://join?u=<url-encoded join URL>`; everything except ASCII letters, digits and `-._~` is percent-encoded. Example:

```text
locale://join?u=http%3A%2F%2F192.168.1.20%3A7420%2Fe%2Fevt_3f9a0c12b7de%23t%3DBwcHBwcHBwcHBwcHBwcHBw%26fp%3DTyqcHnewBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8
```

## 2. Errors and reason codes (§5.2)

Error body (`ErrorBody`): `{"error": {"code", "message", "retryable", "retry_after_ms"?, "received_bytes"?}}`. `received_bytes` is set only for `offset_mismatch`. Example:

```json
{"error":{"code":"offset_mismatch","message":"upload offset mismatch","received_bytes":1048576,"retryable":true}}
```

### Item reason codes (`ItemReason`, open)

| Code | Retryable |
|---|---|
| `type` | no |
| `size` | no |
| `storage_full` | no |
| `abandoned` | yes |
| `stalled` | yes |
| `cancelled` | no |
| `unauthorized` | no |
| `invalid_image` | no |
| `integrity` | yes |
| `duplicate` | no |

### Join failures (`JoinFailure`)

| Code | Sent by | HTTP status | Retryable |
|---|---|---|---|
| `wrong_password` | host | 401 | no |
| `token_invalid` | host | 401 | no |
| `removed` | host | 403 | no |
| `event_ended` | host | 410 | no |
| `browser_not_allowed` | host | 403 | no |
| `rate_limited` | host | 429 | yes |
| `local_network_denied` | client only | — | no |
| `fingerprint_mismatch` | client only | — | no |
| `unreachable` | client only | — | yes |
| `token_expired` | client only | — | no |

### General API codes (`ApiCode`)

| Code | HTTP status | Retryable |
|---|---|---|
| `session_invalid` | 401 | no |
| `role_denied` | 403 | no |
| `not_found` | 404 | no |
| `not_available` | 409 | no |
| `offset_mismatch` | 409 | yes |
| `rate_limited` | 429 | yes |
| `bad_request` | 400 | no |
| `proto_unsupported` | 426 | no |

## 3. Host API (§5.3)

Auth: `Authorization: Bearer <session_token>`; the guest page also gets the token as cookie `locale_session` scoped to `/e/{event_id}`. Every request carries `X-Locale-Proto: 1`.
Bodies are JSON; their schemas are in `schema/`. Host-device-only actions are not on the network API.

| Method | Path | Who | Request | Response |
|---|---|---|---|---|
| `GET` | `/e/{event_id}` | Anyone (plain HTTP) | — | 200 guest web page (HTML) |
| `GET` | `/e/{event_id}/meta` | Anyone | optional header X-Join-Token | 200 MetaResponse |
| `POST` | `/e/{event_id}/join` | Anyone | JoinRequest | 200 JoinResponse |
| `POST` | `/e/{event_id}/session/refresh` | Attendee | no body | 200 SessionRefreshResponse |
| `GET` | `/e/{event_id}/ws` | Attendee | WebSocket upgrade | 101; then ClientMessage / ServerMessage frames |
| `POST` | `/e/{event_id}/items` | uploader, host | CreateItemRequest (idempotent on submission_id) | 200 CreateItemResponse |
| `HEAD` | `/e/{event_id}/uploads/{submission_id}` | Uploader | — | 200, header Upload-Offset |
| `PUT` | `/e/{event_id}/uploads/{submission_id}` | Uploader | headers Upload-Token, Upload-Offset; body = one chunk | 200 UploadChunkResponse; wrong offset → 409 offset_mismatch with received_bytes |
| `DELETE` | `/e/{event_id}/uploads/{submission_id}` | Uploader | — | 204; item → FAILED cancelled |
| `GET` | `/e/{event_id}/submissions/{submission_id}` | Uploader | — | 200 SubmissionStatus |
| `GET` | `/e/{event_id}/items/{id}/preview` | Attendee | — | 200 image/jpeg |
| `GET` | `/e/{event_id}/items/{id}/original` | Attendee | optional Range; ?save=1 counts a download | 200/206 file; headers X-Locale-Sha256, Content-Disposition (original_name) |
| `GET` | `/e/{event_id}/items/{id}/compatible` | Attendee | optional Range; ?save=1 counts a download | 200/206 JPEG; headers X-Locale-Sha256, Content-Disposition (original_name, .jpg) |
| `POST` | `/e/{event_id}/items/{id}/saved` | Attendee (apps) | SavedRequest | 204 |
| `DELETE` | `/e/{event_id}/items/{id}` | Uploader of it, host | — | 204 |
| `POST` | `/e/{event_id}/posts` | host role | CreatePostRequest (text 1–500 chars) | 200 Post |
| `DELETE` | `/e/{event_id}/posts/{id}` | host role | — | 204 |

Headers: `X-Locale-Proto`, `Authorization`, `Upload-Token`, `Upload-Offset`, `X-Locale-Sha256`, `X-Join-Token`, `Content-Disposition`, `Range`.
A download counts only with `?save=1` or `POST …/saved` (DU-19).

Rate limits: uploads 30 per 300000 ms per user (tunable), joins 10 per minute per IP, password attempts 5 per minute per IP.

## 4. Index sync messages (§5.4)

WebSocket JSON text frames tagged by `t`. `hello` must arrive within 5000 ms. Both message enums are open: an unknown `t` parses to `Unknown` and is ignored.

Client → host (`ClientMessage`):

```json
{"session_token":"s3ss10n-t0k3n","t":"hello","version":41}
{"t":"resync","version":41}
{"t":"ping"}
```

Host → client (`ServerMessage`):

```json
{"attendees":[{"client_kind":"ios","display_name":"Leo","id":"0b6f2c1d-3e4a-4b5c-9d6e-7f8091a2b3c4","is_host_device":false,"role":"uploader","verified":true}],"config":{"browser_guests_available":true,"compatible_copy_strategy":"on_demand","desktop_download_all":true,"outbox_config":{"max_age_ms":86400000,"max_attempts":10},"quick_session_enabled":true,"rate_limits":{"upload_window_ms":300000,"uploads_per_window":30},"reachability_test_mode":"full","transfer_config":{"backoff_base_ms":500,"chunk_bytes":1048576,"stall_timeout_ms":120000,"transfer_retries":3}},"event":{"access_type":"open","browser_guests_allowed":true,"created_at":1763180000000,"description":"","ends_at":1763204400000,"expires_at":1763290800000,"extension_count":0,"host_display_name":"Maya","id":"evt_3f9a0c12b7de","kind":"event","links":[],"max_ends_at":1763795000000,"name":"Rooftop","retention_hours":24,"starts_at":1763190000000,"status":"active","storage_limit_bytes":21474836480},"items":[{"available_at":1763200000000,"compatible_same_as_original":false,"content_type":"image/heic","download_count":0,"has_hdr_gain_map":true,"height":3024,"hot_score":0.0,"hot_score_updated_at":1763200000000,"id":"7d1e5f0a-2b3c-4d5e-8f90-a1b2c3d4e5f6","original_name":"IMG_0042.HEIC","sha256":"9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08","size_bytes":3145728,"uploader_id":"0b6f2c1d-3e4a-4b5c-9d6e-7f8091a2b3c4","width":4032}],"posts":[{"author_id":"1c2d3e4f-5a6b-4c7d-8e9f-0a1b2c3d4e5f","created_at":1763200100000,"id":"5a4b3c2d-1e0f-4a9b-8c7d-6e5f4a3b2c1d","kind":"host","text":"Cake at 9"}],"t":"snapshot","version":41}
{"changes":[{"payload":{"id":"7d1e5f0a-2b3c-4d5e-8f90-a1b2c3d4e5f6"},"ts":1763200300000,"type":"item_deleted","version":42}],"t":"changes"}
{"payload":{"id":"7d1e5f0a-2b3c-4d5e-8f90-a1b2c3d4e5f6"},"t":"change","ts":1763200300000,"type":"item_deleted","version":42}
{"item_id":"7d1e5f0a-2b3c-4d5e-8f90-a1b2c3d4e5f6","reason_code":null,"received_bytes":3145728,"retryable":false,"state":"AVAILABLE","submission_id":"c0ffee00-1234-4abc-8def-0123456789ab","t":"upload_state"}
{"expires_at":1763290800000,"kind":"expiring_soon","t":"notice"}
{"reason":"event_ended","t":"bye"}
{"t":"pong"}
```

Bye reasons (open): `removed`, `event_ended`, `session_expired`. Notice kinds (open): `expiring_soon`.

### Change types (`ChangeType`, open)

Envelope `{version, type, payload, ts}`. An unknown `type` is kept as `ChangeBody::Unknown` with its raw payload.

| `type` | Payload example |
|---|---|
| `event_updated` | `{"name":"Rooftop party"}` |
| `event_extended` | `{"ends_at":1763290800000,"new_expires_at":1763377200000,"old_expires_at":1763290800000,"retention_hours":24}` |
| `item_available` | `{"available_at":1763200000000,"compatible_same_as_original":false,"content_type":"image/heic","has_hdr_gain_map":true,"height":3024,"id":"7d1e5f0a-2b3c-4d5e-8f90-a1b2c3d4e5f6","original_name":"IMG_0042.HEIC","sha256":"9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08","size_bytes":3145728,"uploader_id":"0b6f2c1d-3e4a-4b5c-9d6e-7f8091a2b3c4","width":4032}` |
| `item_deleted` | `{"id":"7d1e5f0a-2b3c-4d5e-8f90-a1b2c3d4e5f6"}` |
| `attendee_joined` | `{"client_kind":"ios","display_name":"Leo","id":"0b6f2c1d-3e4a-4b5c-9d6e-7f8091a2b3c4","is_host_device":false,"role":"uploader","verified":true}` |
| `attendee_removed` | `{"id":"0b6f2c1d-3e4a-4b5c-9d6e-7f8091a2b3c4"}` |
| `post_published` | `{"author_id":"1c2d3e4f-5a6b-4c7d-8e9f-0a1b2c3d4e5f","created_at":1763200100000,"id":"5a4b3c2d-1e0f-4a9b-8c7d-6e5f4a3b2c1d","kind":"host","text":"Cake at 9"}` |
| `post_deleted` | `{"id":"5a4b3c2d-1e0f-4a9b-8c7d-6e5f4a3b2c1d"}` |
| `counts_updated` | `[{"download_count":3,"hot_score":2.5,"hot_score_updated_at":1763200200000,"id":"7d1e5f0a-2b3c-4d5e-8f90-a1b2c3d4e5f6"}]` |
| `event_ended` | `{}` |

## 5. App service API (§5.5)

| Method | Path | Request | Response |
|---|---|---|---|
| `POST` | `/v1/auth/start` | AuthStartRequest | 204; emails a 6-digit code valid 10 min |
| `POST` | `/v1/auth/verify` | AuthVerifyRequest | 200 AuthVerifyResponse |
| `POST` | `/v1/credential/refresh` | CredentialRefreshRequest | 200 CredentialRefreshResponse |
| `PATCH` | `/v1/account` | AccountPatchRequest; Authorization: Bearer <refresh_token> | 200 AccountPatchResponse |
| `POST` | `/v1/host-link` | no body; Authorization: Bearer <refresh_token> | 204; emails the desktop download link to the account's address |

Account-scoped requests authenticate with `Authorization: Bearer <refresh_token>`.

**Credential:** `v1.<base64url(payload JSON)>.<base64url(Ed25519 signature)>`. The signature covers the ASCII bytes `v1.<base64url(payload JSON)>`. The payload JSON is compact, fields in this order:

```json
{"kid":"k1","account_id":"2e3f4a5b-6c7d-4e8f-9a0b-1c2d3e4f5a6b","display_name":"Leo","role":"verified","device_pubkey":"CQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQkJCQk","issued_at":1763200000000,"expires_at":1763804800000}
```

**Proof of possession:** `proof` in `POST /join` is `{"timestamp", "signature"}`; the device key signs these 70 bytes:

| Part | Bytes | Encoding |
|---|---|---|
| `"locale-join-v1"` | 14 | ASCII |
| event_id | 16 | ASCII of `evt_` + 12 hex |
| cert_fingerprint | 32 | raw SHA-256 digest |
| timestamp | 8 | Unix ms, big-endian signed 64-bit |

Accepted within ±300000 ms of the host clock.

## 6. mDNS (§5.6)

Service type `_locale._tcp.local.`, instance name = event ID. Advertised for `open` and `password` events only. TXT keys: `v`, `id`, `name`, `host`, `access`, `st`, `en`, `fp`. Example:

```text
v=1
id=evt_3f9a0c12b7de
name=Rooftop
host=Maya
access=open
st=1763190000000
en=1763204400000
fp=4F2A9C1E77B0
```

## 7. Enums (§6)

Open enums map unknown values to `Unknown` (receive-only, never sent). Closed enums reject unknown values.

| Enum | Kind | Values | Why |
|---|---|---|---|
| `AccessType` | closed | `open`, `invite`, `password` | access control |
| `EventStatus` | closed | `active`, `paused`, `ended` |  |
| `EventKind` | closed | `event`, `quick` |  |
| `Role` | closed | `host`, `uploader`, `viewer` | permissions |
| `ClientKind` | closed | `ios`, `android`, `desktop`, `web` |  |
| `SessionScope` | closed | `full`, `read` | permissions |
| `ItemState` | closed | `PENDING`, `UPLOADING`, `VALIDATING`, `AVAILABLE`, `FAILED`, `DELETED` | state machine |
| `PostKind` | closed | `host`, `system` |  |
| `DownloadVersion` | closed | `original`, `compatible` |  |
| `CredentialRole` | closed | `verified` |  |
| `ItemReason` | open | `type`, `size`, `storage_full`, `abandoned`, `stalled`, `cancelled`, `unauthorized`, `invalid_image`, `integrity`, `duplicate` | unknown = final, not retryable |
| `JoinFailure` | closed | `wrong_password`, `token_invalid`, `removed`, `event_ended`, `browser_not_allowed`, `rate_limited`, `local_network_denied`, `fingerprint_mismatch`, `unreachable`, `token_expired` | travels as the string `error.code` |
| `ApiCode` | closed | `session_invalid`, `role_denied`, `not_found`, `not_available`, `offset_mismatch`, `rate_limited`, `bad_request`, `proto_unsupported` | travels as the string `error.code` |
| `ChangeType` | open | `event_updated`, `event_extended`, `item_available`, `item_deleted`, `attendee_joined`, `attendee_removed`, `post_published`, `post_deleted`, `counts_updated`, `event_ended` | unknown changes are skipped |
| `ClientMessage.t` | open | `hello`, `resync`, `ping` | unknown frames are ignored |
| `ServerMessage.t` | open | `snapshot`, `changes`, `change`, `upload_state`, `notice`, `bye`, `pong` | unknown frames are ignored |
| `ByeReason` | open | `removed`, `event_ended`, `session_expired` | socket closes anyway |
| `NoticeKind` | open | `expiring_soon` | informational |
| `ReachabilityTestMode` | open | `full` | unknown → compiled default |
| `CompatibleCopyStrategy` | open | `on_demand` | unknown → compiled default |

## 8. Item state machine (§7)

Any transition not in this table is illegal. `DELETED` is a tombstone in the change log.

| From | To | Trigger | Who | Reason codes | Retryable |
|---|---|---|---|---|---|
| (none) | PENDING | Submit passes User Validation | Host |  |  |
| PENDING | UPLOADING | Basic Content Precheck passes, storage available | Host |  |  |
| PENDING | FAILED | Precheck denied | Host | `type`, `size`, `storage_full` | No |
| PENDING | FAILED | No progress for 10 min | Sweeper | `abandoned` | Yes |
| UPLOADING | VALIDATING | Last chunk received | Host |  |  |
| UPLOADING | FAILED | No chunk for 2 min | Sweeper | `stalled` | Yes |
| UPLOADING | FAILED | Uploader cancels | Host on request | `cancelled` | No |
| UPLOADING | FAILED | Token invalid | Host | `unauthorized` | No |
| FAILED | UPLOADING | Same submission ID resubmitted, reason retryable | Host |  |  |
| VALIDATING | AVAILABLE | Integrity and Server Content Validation pass | Host |  |  |
| VALIDATING | FAILED | Validation denied | Host | `invalid_image` | No |
| VALIDATING | FAILED | Integrity retries exceeded | Host | `integrity` | Yes |
| VALIDATING | UPLOADING | Integrity mismatch, fewer than 3 so far: bytes discarded, offset 0 (DU-13) | Host |  |  |
| AVAILABLE | DELETED | Uploader or host deletes, or event expires | Host |  |  |

## 9. Configuration (§10, §11)

`Config` is frozen at event start and sent at join and in the snapshot. Remote values outside the bounds, of the wrong type, or unknown enum values are ignored. Compiled defaults:

```json
{
  "browser_guests_available": true,
  "compatible_copy_strategy": "on_demand",
  "desktop_download_all": true,
  "outbox_config": {
    "max_age_ms": 86400000,
    "max_attempts": 10
  },
  "quick_session_enabled": true,
  "rate_limits": {
    "upload_window_ms": 300000,
    "uploads_per_window": 30
  },
  "reachability_test_mode": "full",
  "transfer_config": {
    "backoff_base_ms": 500,
    "chunk_bytes": 1048576,
    "stall_timeout_ms": 120000,
    "transfer_retries": 3
  }
}
```

| Key | Default | Min | Max | Unit |
|---|---|---|---|---|
| `transfer_config.chunk_bytes` | 1048576 | 262144 | 4194304 | bytes |
| `transfer_config.stall_timeout_ms` | 120000 | 30000 | 600000 | ms |
| `transfer_config.transfer_retries` | 3 | 1 | 10 | count |
| `transfer_config.backoff_base_ms` | 500 | 100 | 10000 | ms |
| `outbox_config.max_attempts` | 10 | 1 | 50 | count |
| `outbox_config.max_age_ms` | 86400000 | 3600000 | 604800000 | ms |
| `rate_limits.uploads_per_window` | 30 | 1 | 1000 | count |
| `rate_limits.upload_window_ms` | 300000 | 60000 | 3600000 | ms |

Fixed constants (`proto::config`):

| Constant | Value | Unit |
|---|---|---|
| `MAX_PHOTO_BYTES` | 52428800 | bytes |
| `DEFAULT_STORAGE_LIMIT_BYTES` | 21474836480 | bytes |
| `QUICK_SESSION_STORAGE_LIMIT_BYTES` | 5368709120 | bytes |
| `MAX_EVENT_LIFETIME_MS` | 604800000 | ms |
| `PENDING_TIMEOUT_MS` | 600000 | ms |
| `FAILED_CLEANUP_MS` | 3600000 | ms |
| `SWEEPER_INTERVAL_MS` | 60000 | ms |
| `SESSION_TTL_MS` | 43200000 | ms |
| `BROWSER_SESSION_TTL_MS` | 7200000 | ms |
| `CREDENTIAL_LIFETIME_MS` | 604800000 | ms |
| `CREDENTIAL_REFRESH_REMAINING_MS` | 172800000 | ms |
| `COUNT_BATCH_INTERVAL_MS` | 30000 | ms |
| `HOT_SCORE_HALF_LIFE_MS` | 900000 | ms |
| `PREVIEW_MAX_EDGE_PX` | 1024 | px |
| `PREVIEW_JPEG_QUALITY` | 80 | quality |
| `COMPATIBLE_JPEG_QUALITY` | 92 | quality |
| `MAX_INTEGRITY_ATTEMPTS` | 3 | count |
| `POST_TEXT_MAX_CHARS` | 500 | chars |
| `LOGIN_CODE_DIGITS` | 6 | digits |
| `LOGIN_CODE_TTL_MS` | 600000 | ms |


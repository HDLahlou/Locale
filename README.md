# Locale

Locale turns nearby phones into a temporary mesh of nodes, so people in the same place can share files at full quality, securely, on any device.

## Goal

Share photos, video, and files with people near you without uploading to a cloud server, without compression, and without caring whether they use an iPhone, an Android phone, or a computer.

## How it works

- **Events are temporary pages hosted by participants.** There is no central cloud host; the people at the event are the infrastructure.
- **Nearby users become nodes.** A node can host content, relay data for others, and hold backups.
- **Transfers use the best path available.** Wi-Fi on the same network first, then Bluetooth peer to peer, then hops through other nearby nodes when the recipient is out of direct range.
- **An index tracks where every file lives.** If a node drops, the index points to a backup so content stays available.

## Why it exists

- **Original quality.** Files are not compressed to save cloud storage.
- **Privacy from the cloud.** Event data lives on participants' devices.
- **Works without service.** Festivals, hikes, classrooms, and anywhere reception is poor.
- **Temporary by default.** Not everything needs to be permanent or on the internet.
- **Device agnostic.** Apple, Android, and computers share in the same event.

## Security approach

- Only verified accounts can upload; guests can view and download.
- Transfers run over encrypted, authenticated sessions with expiring tokens, so relay nodes only carry ciphertext.
- Content is scanned, and location metadata is stripped, before it is shared.
- Authoritative checks never trust the client.

## Status

In design.

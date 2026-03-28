#!/usr/bin/env node
// generate-keys.mjs
// Generates an Ed25519 key pair for signing the registry
// Usage: node generate-keys.mjs

import { generateKeyPairSync, createPublicKey } from "crypto";
import { writeFileSync } from "fs";

const { publicKey, privateKey } = generateKeyPairSync("ed25519");

// Extract raw bytes (skip PEM headers)
const pem = privateKey.export({ type: "pkcs8", format: "pem" });
const base64 = pem.split("\n").filter(l => !l.startsWith("-----")).join("");
const privateKeyHex = Buffer.from(base64, "base64").toString("hex");

// Extract public key bytes
const pubPem = publicKey.export({ type: "spki", format: "pem" });
const pubBase64 = pubPem.split("\n").filter(l => !l.startsWith("-----")).join("");
const publicKeyHex = Buffer.from(pubBase64, "base64").toString("hex");

// Take last 64 chars (32 bytes) for the public key
const pubKey32 = publicKeyHex.slice(-64);

writeFileSync("private_key.hex", privateKeyHex);
writeFileSync("public_key.hex", pubKey32);

console.log("✅ Keys generated:");
console.log(`   Private key: private_key.hex (KEEP SECRET)`);
console.log(`   Public key:  ${pubKey32}`);
console.log("\n⚠️  Store private_key.hex safely and delete it from the repo!");

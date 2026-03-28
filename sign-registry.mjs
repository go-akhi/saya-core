#!/usr/bin/env node
// sign-registry.mjs
// Usage: node sign-registry.mjs plugins.json private_key.hex

import { readFileSync, writeFileSync } from "fs";
import { createPrivateKey, sign } from "crypto";

const [jsonFile, keyFile] = process.argv.slice(2);

if (!jsonFile || !keyFile) {
  console.error("Usage: node sign-registry.mjs <plugins.json> <private_key.hex>");
  process.exit(1);
}

// Read files
const json = JSON.parse(readFileSync(jsonFile, "utf-8"));
const privateKeyHex = readFileSync(keyFile, "utf-8").trim();

// Strip signature fields for signing
const { signature, public_key, ...stripped } = json;

// Sort keys deterministically
const canonicalJson = JSON.stringify(stripped, Object.keys(stripped).sort());

// Convert hex private key to PEM
const privateKeyBytes = Buffer.from(privateKeyHex, "hex");
const pemHeader = "-----BEGIN PRIVATE KEY-----\n";
const pemFooter = "\n-----END PRIVATE KEY-----\n";
const base64Key = privateKeyBytes.toString("base64");
const pem = pemHeader + base64Key.match(/.{1,64}/g).join("\n") + pemFooter;

// Create key object
const keyObj = createPrivateKey({
  key: pem,
  format: "pem",
});

// Sign
const signatureBuffer = sign(null, Buffer.from(canonicalJson), keyObj);
const signatureHex = signatureBuffer.toString("hex");

// Update JSON
json.signature = signatureHex;

// Write
writeFileSync(jsonFile, JSON.stringify(json, null, 2) + "\n");

console.log(`✅ Signed ${jsonFile}`);
console.log(`   Signature: ${signatureHex.slice(0, 16)}...`);
console.log(`   Public key: ${public_key}`);

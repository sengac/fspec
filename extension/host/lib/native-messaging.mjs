/**
 * Native Messaging Protocol
 *
 * Chrome native messaging uses a simple framing protocol:
 * - 4-byte little-endian unsigned 32-bit integer (message length)
 * - Raw JSON bytes (UTF-8)
 *
 * Limits (asymmetric):
 * - Outgoing (host → extension): 1 MB (1024 * 1024 bytes)
 *   Chrome's kMaximumNativeMessageSize in native_message_process_host.cc
 * - Incoming (extension → host): 64 MiB (64 * 1024 * 1024 bytes)
 *   Chrome's kMaximumExtensionMessageSize in messaging_util.cc
 */

const MAX_OUTGOING_MESSAGE_SIZE = 1024 * 1024;           // 1 MB (host → extension)
const MAX_INCOMING_MESSAGE_SIZE = 64 * 1024 * 1024;      // 64 MiB (extension → host)

/**
 * Encode a JavaScript object as a native messaging frame.
 * Enforces the 1 MB outgoing limit (host → extension).
 * @param {object} message - The message to encode
 * @returns {Buffer} - 4-byte length prefix + JSON bytes
 */
export function encodeNativeMessage(message) {
  const jsonStr = JSON.stringify(message);
  const jsonBytes = Buffer.from(jsonStr, 'utf-8');

  if (jsonBytes.length > MAX_OUTGOING_MESSAGE_SIZE) {
    throw new Error(`Message exceeds max size: ${jsonBytes.length} > ${MAX_OUTGOING_MESSAGE_SIZE}`);
  }

  const lengthPrefix = Buffer.alloc(4);
  lengthPrefix.writeUInt32LE(jsonBytes.length, 0);

  return Buffer.concat([lengthPrefix, jsonBytes]);
}

/**
 * Decode a native messaging frame back to a JavaScript object.
 * Uses the incoming limit (64 MiB) since this decodes messages from the extension.
 * @param {Buffer} buffer - The raw frame (4-byte length prefix + JSON bytes)
 * @returns {object} - The decoded message
 */
export function decodeNativeMessage(buffer) {
  if (buffer.length < 4) {
    throw new Error('Buffer too short: need at least 4 bytes for length prefix');
  }

  const length = buffer.readUInt32LE(0);

  if (length > MAX_INCOMING_MESSAGE_SIZE) {
    throw new Error(`Message length exceeds max: ${length} > ${MAX_INCOMING_MESSAGE_SIZE}`);
  }

  if (buffer.length < 4 + length) {
    throw new Error(`Incomplete message: expected ${4 + length} bytes, got ${buffer.length}`);
  }

  const jsonBytes = buffer.subarray(4, 4 + length);
  return JSON.parse(jsonBytes.toString('utf-8'));
}

/**
 * Create a stream reader that reads native messaging frames from a readable stream.
 * Handles partial reads, buffering, and oversized message skipping.
 *
 * Uses the 64 MiB incoming limit since messages flow from extension → host.
 * When an oversized message is encountered, skips exactly its bytes to
 * preserve stream integrity for subsequent messages.
 *
 * @param {import('stream').Readable} inputStream - The stream to read from
 * @param {(message: object) => void} onMessage - Callback for each decoded message
 * @returns {{ stop: () => void }} - Control handle
 */
export function createNativeMessageReader(inputStream, onMessage) {
  let buffer = Buffer.alloc(0);
  let stopped = false;
  /** Number of bytes remaining to skip for an oversized message */
  let skipRemaining = 0;

  function processBuffer() {
    // First, handle any pending skip from an oversized message
    if (skipRemaining > 0) {
      if (buffer.length >= skipRemaining) {
        buffer = buffer.subarray(skipRemaining);
        skipRemaining = 0;
        // Continue processing — there may be valid messages after the skipped one
      } else {
        // Not enough data to finish skipping — consume what we have
        skipRemaining -= buffer.length;
        buffer = Buffer.alloc(0);
        return;
      }
    }

    while (buffer.length >= 4 && !stopped) {
      const length = buffer.readUInt32LE(0);

      if (length > MAX_INCOMING_MESSAGE_SIZE) {
        // Oversized message — skip exactly 4 + length bytes to preserve stream integrity
        const totalFrameSize = 4 + length;
        if (buffer.length >= totalFrameSize) {
          // We have the entire oversized message — skip it
          buffer = buffer.subarray(totalFrameSize);
          // Continue processing — there may be more messages
          continue;
        } else {
          // We don't have the full oversized message yet
          // Skip the header (4 bytes) and track remaining body bytes to skip
          skipRemaining = length - (buffer.length - 4);
          buffer = Buffer.alloc(0);
          return;
        }
      }

      if (buffer.length < 4 + length) {
        // Incomplete message — wait for more data
        break;
      }

      const jsonBytes = buffer.subarray(4, 4 + length);
      buffer = buffer.subarray(4 + length);

      try {
        const message = JSON.parse(jsonBytes.toString('utf-8'));
        onMessage(message);
      } catch {
        // Malformed JSON — skip
      }
    }
  }

  function onData(chunk) {
    if (stopped) { return; }
    buffer = Buffer.concat([buffer, chunk]);
    processBuffer();
  }

  inputStream.on('data', onData);

  return {
    stop() {
      stopped = true;
      inputStream.removeListener('data', onData);
    },
  };
}

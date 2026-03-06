/**
 * Native Messaging Protocol
 *
 * Chrome native messaging uses a simple framing protocol:
 * - 4-byte little-endian unsigned 32-bit integer (message length)
 * - Raw JSON bytes (UTF-8)
 *
 * Max message size: 1MB (1024 * 1024 bytes)
 */

const MAX_MESSAGE_SIZE = 1024 * 1024;

/**
 * Encode a JavaScript object as a native messaging frame.
 * @param {object} message - The message to encode
 * @returns {Buffer} - 4-byte length prefix + JSON bytes
 */
export function encodeNativeMessage(message) {
  const jsonStr = JSON.stringify(message);
  const jsonBytes = Buffer.from(jsonStr, 'utf-8');

  if (jsonBytes.length > MAX_MESSAGE_SIZE) {
    throw new Error(`Message exceeds max size: ${jsonBytes.length} > ${MAX_MESSAGE_SIZE}`);
  }

  const lengthPrefix = Buffer.alloc(4);
  lengthPrefix.writeUInt32LE(jsonBytes.length, 0);

  return Buffer.concat([lengthPrefix, jsonBytes]);
}

/**
 * Decode a native messaging frame back to a JavaScript object.
 * @param {Buffer} buffer - The raw frame (4-byte length prefix + JSON bytes)
 * @returns {object} - The decoded message
 */
export function decodeNativeMessage(buffer) {
  if (buffer.length < 4) {
    throw new Error('Buffer too short: need at least 4 bytes for length prefix');
  }

  const length = buffer.readUInt32LE(0);

  if (length > MAX_MESSAGE_SIZE) {
    throw new Error(`Message length exceeds max: ${length} > ${MAX_MESSAGE_SIZE}`);
  }

  if (buffer.length < 4 + length) {
    throw new Error(`Incomplete message: expected ${4 + length} bytes, got ${buffer.length}`);
  }

  const jsonBytes = buffer.subarray(4, 4 + length);
  return JSON.parse(jsonBytes.toString('utf-8'));
}

/**
 * Create a stream reader that reads native messaging frames from a readable stream.
 * Handles partial reads and buffering.
 *
 * @param {import('stream').Readable} inputStream - The stream to read from
 * @param {(message: object) => void} onMessage - Callback for each decoded message
 * @returns {{ stop: () => void }} - Control handle
 */
export function createNativeMessageReader(inputStream, onMessage) {
  let buffer = Buffer.alloc(0);
  let stopped = false;

  function processBuffer() {
    while (buffer.length >= 4 && !stopped) {
      const length = buffer.readUInt32LE(0);

      if (length > MAX_MESSAGE_SIZE) {
        // Invalid frame — skip
        buffer = Buffer.alloc(0);
        break;
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

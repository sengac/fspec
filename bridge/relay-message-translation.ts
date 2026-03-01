/**
 * Relay Bridge Endpoint — Message Translation
 *
 * Translates between relay protocol (camelCase, {data:{...}} envelope)
 * and fspec InboundMessage format (flat fields, 'control' type).
 *
 * BRIDGE-015: Platform-Agnostic Relay Bridge Endpoint
 */

import type {
  RelayMessage,
  FspecInboundMessage,
  RelayEndpointConfig,
  ConfigValidationResult,
} from './relay-types';

// ============================================================================
// Config Validation
// ============================================================================

export function validateConfig(
  configInput: Partial<RelayEndpointConfig>
): ConfigValidationResult {
  const errors: string[] = [];

  if (!configInput.relayUrl) {
    errors.push('RELAY_URL is required');
  }
  if (!configInput.channelId) {
    errors.push('RELAY_CHANNEL_ID is required');
  }
  if (!configInput.apiKey) {
    errors.push('RELAY_API_KEY is required');
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

// ============================================================================
// Relay → fspec Translation
// ============================================================================

/**
 * Translate relay input message to fspec flat InboundMessage format.
 * Unwraps the data envelope so message and images are top-level fields.
 */
export function translateRelayInputToFspec(
  relayMsg: RelayMessage
): FspecInboundMessage {
  const data = relayMsg.data || {};
  return {
    type: 'input',
    session_id: relayMsg.session_id || '',
    message: data.message as string | undefined,
    images: data.images as
      | Array<{ data: string; media_type: string }>
      | undefined,
  };
}

/**
 * Translate relay sessionControl to fspec control format.
 * Renames type from 'sessionControl' to 'control' and unwraps data envelope.
 */
export function translateRelaySessionControlToFspec(
  relayMsg: RelayMessage
): FspecInboundMessage {
  const data = relayMsg.data || {};
  return {
    type: 'control',
    session_id: relayMsg.session_id || '',
    action: data.action as string | undefined,
    response: data.response as string | undefined,
  };
}

/**
 * Translate relay command message to fspec flat InboundMessage format.
 * Unwraps the data envelope and serialises args to args_json string.
 * BRIDGE-018: Commands flow through the codelet bridge WebSocket to bridge_relay.rs.
 */
export function translateRelayCommandToFspec(
  relayMsg: RelayMessage
): FspecInboundMessage {
  const data = relayMsg.data || {};
  return {
    type: 'command',
    session_id: relayMsg.session_id || '',
    message: '',
    request_id: relayMsg.request_id || '',
    command: data.command as string,
    args_json: JSON.stringify(data.args || {}),
  };
}

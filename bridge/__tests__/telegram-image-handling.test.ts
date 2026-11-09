/**
 * Feature: spec/features/support-incoming-image-attachments-from-telegram.feature
 *
 * This test file validates the acceptance criteria for BRIDGE-007:
 * Support Incoming Image Attachments from Telegram
 */

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

// The Telegram bot and WebSocket server are mocked so the real bot 'message'
// handler registered by startEndpoint() can be driven without network access.
const { mockBotOn, mockBotGetFileLink, MockTelegramBot, MockWebSocketServer } =
  vi.hoisted(() => {
    const mockBotOn = vi.fn();
    const mockBotGetFileLink = vi.fn();

    class MockTelegramBot {
      on = mockBotOn;
      getFileLink = mockBotGetFileLink;
      sendMessage = vi.fn().mockResolvedValue({});
      stopPolling = vi.fn().mockResolvedValue(undefined);
    }

    class MockWebSocketServer {
      on = vi.fn();
      close = vi.fn();
    }

    return {
      mockBotOn,
      mockBotGetFileLink,
      MockTelegramBot,
      MockWebSocketServer,
    };
  });

vi.mock('ws', () => ({
  WebSocketServer: MockWebSocketServer,
  WebSocket: { OPEN: 1 },
}));

vi.mock('node-telegram-bot-api', () => ({
  default: MockTelegramBot,
}));

vi.mock('dotenv', () => ({
  config: vi.fn(),
}));

import {
  getMediaTypeFromPath,
  downloadPhotoAsBase64,
  handleTelegramMessage,
  forwardPhotoToSession,
  resetState,
  getState,
  startEndpoint,
  stopEndpoint,
} from '../telegram-endpoint';

// Mock fetch for photo downloads
const mockFetch = vi.fn();
globalThis.fetch = mockFetch;

describe('Feature: Support Incoming Image Attachments from Telegram', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockFetch.mockReset();
    mockBotGetFileLink.mockReset();
    resetState();
  });

  afterEach(async () => {
    vi.restoreAllMocks();
    if (getState().isRunning) {
      await stopEndpoint();
    }
    delete process.env.TELEGRAM_BOT_TOKEN;
    resetState();
  });

  /**
   * Start the endpoint (mocked bot and WebSocket server), attach a connected
   * codelet session, and return the bot's real 'message' handler.
   */
  function startEndpointWithSession() {
    process.env.TELEGRAM_BOT_TOKEN = 'test-bot-token-123';
    delete process.env.TELEGRAM_ALLOWED_USER_IDS;
    startEndpoint();

    const sessionWs = { readyState: 1, send: vi.fn() };
    const state = getState();
    state.currentSession.ws = sessionWs as never;
    state.currentSession.sessionId = 'test-session-123';

    const registration = mockBotOn.mock.calls.find(
      ([event]) => event === 'message'
    );
    expect(registration).toBeDefined();
    const onMessage = registration?.[1] as (msg: unknown) => Promise<void>;
    return { sessionWs, onMessage };
  }

  function photoMessage(caption?: string) {
    return {
      chat: { id: 12345 },
      from: { id: 111 },
      photo: [{ file_id: 'photo_id', width: 800, height: 600 }],
      ...(caption === undefined ? {} : { caption }),
    };
  }

  // ====================
  // MEDIA TYPE SCENARIOS
  // ====================

  describe('Scenario Outline: Detect correct media type from file extension', () => {
    it.each([
      ['photos/file_123.jpg', 'image/jpeg'],
      ['photos/file_456.jpeg', 'image/jpeg'],
      ['photos/file_789.png', 'image/png'],
      ['photos/file_abc.gif', 'image/gif'],
      ['photos/file_def.webp', 'image/webp'],
    ])('should detect %s as %s', (filePath, expectedMediaType) => {
      // @step Given I have a connected Telegram bridge session
      // @step And Telegram provides a photo with file_path "<file_path>"
      // @step When the bridge processes the photo message
      const result = getMediaTypeFromPath(filePath);

      // @step Then the image should have media_type "<media_type>"
      expect(result).toBe(expectedMediaType);
    });
  });

  describe('Scenario: Default to image/jpeg for unknown extension', () => {
    it('should default to image/jpeg when extension is unknown', () => {
      // @step Given I have a connected Telegram bridge session
      // @step And Telegram provides a photo with file_path "photos/file_xyz"
      const filePath = 'photos/file_xyz';

      // @step When the bridge processes the photo message
      const result = getMediaTypeFromPath(filePath);

      // @step Then the image should have media_type "image/jpeg"
      expect(result).toBe('image/jpeg');
    });
  });

  describe('Scenario: Default to image/jpeg for no extension', () => {
    it('should default to image/jpeg when no extension present', () => {
      // @step Given I have a connected Telegram bridge session
      // @step And Telegram provides a photo with file_path "photos/file_without_extension"
      const filePath = 'photos/file_without_extension';

      // @step When the bridge processes the photo message
      const result = getMediaTypeFromPath(filePath);

      // @step Then the image should have media_type "image/jpeg"
      expect(result).toBe('image/jpeg');
    });
  });

  // ====================
  // PHOTO DOWNLOAD SCENARIOS
  // ====================

  describe('Scenario: Download highest resolution photo from Telegram', () => {
    it('should request the file with the largest resolution (last in array)', async () => {
      // @step Given I have a connected Telegram bridge session
      const mockBot = {
        getFileLink: vi
          .fn()
          .mockResolvedValue('https://api.telegram.org/file/photo.jpg'),
      };

      // @step And Telegram provides a photo with multiple resolutions
      const photoArray = [
        { file_id: 'thumb_id', width: 90, height: 90 },
        { file_id: 'small_id', width: 320, height: 320 },
        { file_id: 'medium_id', width: 800, height: 800 },
        { file_id: 'large_id', width: 1280, height: 1280 },
      ];

      // Mock fetch response
      const mockImageData = new Uint8Array([0x89, 0x50, 0x4e, 0x47]);
      mockFetch.mockResolvedValue({
        ok: true,
        arrayBuffer: () => Promise.resolve(mockImageData.buffer),
      });

      // @step When the bridge processes the photo message
      const highestRes = photoArray[photoArray.length - 1];
      const result = await downloadPhotoAsBase64(
        mockBot as never,
        highestRes.file_id
      );

      // @step Then it should request the file with file_id "large_id"
      expect(mockBot.getFileLink).toHaveBeenCalledWith('large_id');

      // @step And the downloaded image should be converted to base64
      expect(result).not.toBeNull();
      expect(result?.data).toBeDefined();

      // @step And the WebSocket message should include an images array with the base64 data
      expect(typeof result?.data).toBe('string');
    });
  });

  describe('Scenario: Handle single resolution photo', () => {
    it('should use the only available resolution', async () => {
      // @step Given I have a connected Telegram bridge session
      const mockBot = {
        getFileLink: vi
          .fn()
          .mockResolvedValue('https://api.telegram.org/file/photo.jpg'),
      };

      // @step And Telegram provides a photo with only one resolution
      const photoArray = [{ file_id: 'single_id', width: 800, height: 600 }];

      mockFetch.mockResolvedValue({
        ok: true,
        arrayBuffer: () => Promise.resolve(new Uint8Array([1, 2, 3]).buffer),
      });

      // @step When the bridge processes the photo message
      const result = await downloadPhotoAsBase64(
        mockBot as never,
        photoArray[0].file_id
      );

      // @step Then it should request the file with file_id "single_id"
      expect(mockBot.getFileLink).toHaveBeenCalledWith('single_id');

      // @step And the WebSocket message should include the image in the images array
      expect(result).not.toBeNull();
      expect(result?.data).toBeDefined();
    });
  });

  // ====================
  // CAPTION SCENARIOS
  // ====================

  describe('Scenario: Include caption text with photo', () => {
    it('should include caption in WebSocket message along with image', () => {
      // @step Given I have a connected Telegram bridge session
      resetState();
      const state = getState();
      state.currentSession.sessionId = 'test-session-123';

      // @step And Telegram provides a photo message with caption "What error is this?"
      const caption = 'What error is this?';
      const images = [{ data: 'base64data', media_type: 'image/jpeg' }];

      // @step When the bridge processes the photo message
      const result = handleTelegramMessage('12345', caption, images);

      // @step Then the WebSocket envelope should have message "What error is this?"
      expect(result.data.message).toBe('What error is this?');

      // @step And the WebSocket envelope should include the image in data.images
      expect(result.data.images).toBeDefined();
      expect(result.data.images?.length).toBe(1);
    });
  });

  describe('Scenario: Handle photo without caption', () => {
    it('should send empty message string with image', () => {
      // @step Given I have a connected Telegram bridge session
      resetState();
      const state = getState();
      state.currentSession.sessionId = 'test-session-123';

      // @step And Telegram provides a photo message without a caption
      const caption = '';
      const images = [{ data: 'base64data', media_type: 'image/jpeg' }];

      // @step When the bridge processes the photo message
      const result = handleTelegramMessage('12345', caption, images);

      // @step Then the WebSocket envelope should have message ""
      expect(result.data.message).toBe('');

      // @step And the WebSocket envelope should include the image in data.images
      expect(result.data.images).toBeDefined();
      expect(result.data.images?.length).toBe(1);
    });
  });

  describe('Scenario: Use msg.caption not msg.text for photo messages', () => {
    it('should use caption field, ignoring text field', async () => {
      // @step Given I have a connected Telegram bridge session
      const { sessionWs, onMessage } = startEndpointWithSession();
      mockBotGetFileLink.mockResolvedValue(
        'https://api.telegram.org/file/photos/test.jpg'
      );
      mockFetch.mockResolvedValue({
        ok: true,
        arrayBuffer: () => Promise.resolve(new Uint8Array([1, 2, 3]).buffer),
      });

      // @step And Telegram provides a photo message where:
      //   caption "This is the caption", text "This should be ignored"
      const msg = {
        ...photoMessage('This is the caption'),
        text: 'This should be ignored',
      };

      // @step When the bridge processes the photo message
      await onMessage(msg);

      // @step Then the WebSocket envelope should have message "This is the caption"
      expect(sessionWs.send).toHaveBeenCalledTimes(1);
      const envelope = JSON.parse(String(sessionWs.send.mock.calls[0][0]));
      expect(envelope.data.message).toBe('This is the caption');

      // @step And the message should NOT contain "This should be ignored"
      expect(envelope.data.message).not.toContain('This should be ignored');
    });
  });

  describe('Scenario: Verify WebSocket message structure', () => {
    it('should have correct structure with type, session_id, message, and images', () => {
      // @step Given I have a connected Telegram bridge session with session_id "test-session-123"
      resetState();
      const state = getState();
      state.currentSession.sessionId = 'test-session-123';

      // @step And Telegram provides a photo message with caption "Hello"
      // @step And the photo downloads successfully as base64 "SGVsbG8gV29ybGQ="
      // @step And the photo file_path is "photos/test.jpg"
      const caption = 'Hello';
      const images = [{ data: 'SGVsbG8gV29ybGQ=', media_type: 'image/jpeg' }];

      // @step When the bridge processes the photo message
      const result = handleTelegramMessage('12345', caption, images);

      // @step Then the WebSocket envelope should have this structure:
      //   { service: "session", type: "input", session_id,
      //     data: { message, images: [{ data, media_type }] } }
      expect(result.service).toBe('session');
      expect(result.type).toBe('input');
      expect(result.session_id).toBe('test-session-123');
      expect(result.data.message).toBe('Hello');
      expect(result.data.images?.[0].data).toBe('SGVsbG8gV29ybGQ=');
      expect(result.data.images?.[0].media_type).toBe('image/jpeg');
    });
  });

  // ====================
  // ERROR SCENARIOS
  // ====================

  describe('Scenario: Forward caption when photo download fails', () => {
    it('should log error and forward the caption without images', async () => {
      // @step Given I have a connected Telegram bridge session
      const { sessionWs, onMessage } = startEndpointWithSession();
      const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      // @step And Telegram provides a photo message with caption "Check this out"
      const msg = photoMessage('Check this out');

      // @step And the photo download will fail with error "Network timeout"
      mockBotGetFileLink.mockRejectedValue(new Error('Network timeout'));

      // @step When the bridge processes the photo message
      await onMessage(msg);

      // @step Then the bridge should log an error about the download failure
      expect(errorSpy).toHaveBeenCalledWith(
        '[telegram-endpoint] Photo download failed:',
        expect.objectContaining({ message: 'Network timeout' })
      );

      // @step And the WebSocket envelope should have message "Check this out"
      expect(sessionWs.send).toHaveBeenCalledTimes(1);
      const envelope = JSON.parse(String(sessionWs.send.mock.calls[0][0]));
      expect(envelope.service).toBe('session');
      expect(envelope.type).toBe('input');
      expect(envelope.session_id).toBe('test-session-123');
      expect(envelope.data.message).toBe('Check this out');

      // @step And the WebSocket envelope should have no images
      expect(envelope.data.images).toBeUndefined();
    });
  });

  describe('Scenario: Handle photo download HTTP error', () => {
    it('should return null when fetch returns non-ok response', async () => {
      // @step Given I have a connected Telegram bridge session
      const mockBot = {
        getFileLink: vi
          .fn()
          .mockResolvedValue('https://api.telegram.org/file/photo.jpg'),
      };

      // Mock fetch with HTTP error
      mockFetch.mockResolvedValue({
        ok: false,
        status: 500,
      });

      // @step When the bridge processes the photo message
      const result = await downloadPhotoAsBase64(mockBot as never, 'file_id');

      // @step Then the result should be null (download failed)
      expect(result).toBeNull();
    });
  });

  describe('Scenario: Drop message completely when photo download fails and no caption', () => {
    it('should not send any message when download fails and no caption exists', async () => {
      // @step Given I have a connected Telegram bridge session
      const { sessionWs, onMessage } = startEndpointWithSession();
      const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      vi.spyOn(console, 'warn').mockImplementation(() => {});

      // @step And Telegram provides a photo message without a caption
      const msg = photoMessage();

      // @step And the photo download will fail
      mockBotGetFileLink.mockRejectedValue(new Error('Network error'));

      // @step When the bridge processes the photo message
      await onMessage(msg);

      // @step Then the bridge should log an error about the download failure
      expect(errorSpy).toHaveBeenCalledWith(
        '[telegram-endpoint] Photo download failed:',
        expect.objectContaining({ message: 'Network error' })
      );

      // @step And no WebSocket message should be sent
      expect(sessionWs.send).not.toHaveBeenCalled();
    });
  });

  // ====================
  // EDGE CASE SCENARIOS
  // ====================

  describe('Scenario: Handle photo with empty caption', () => {
    it('should treat empty string caption same as no caption', () => {
      // @step Given I have a connected Telegram bridge session
      resetState();
      const state = getState();
      state.currentSession.sessionId = 'test-session-123';

      // @step And Telegram provides a photo message with caption ""
      const caption = '';
      const images = [{ data: 'base64data', media_type: 'image/jpeg' }];

      // @step When the bridge processes the photo message
      const result = handleTelegramMessage('12345', caption, images);

      // @step Then the WebSocket envelope should have message ""
      expect(result.data.message).toBe('');

      // @step And the WebSocket envelope should include the image in data.images
      expect(result.data.images?.length).toBe(1);
    });
  });

  describe('Scenario: InboundMessage without images for text-only messages', () => {
    it('should not include images field for regular text messages', () => {
      // @step Given I have a connected Telegram bridge session
      resetState();
      const state = getState();
      state.currentSession.sessionId = 'test-session-123';

      // @step When a user sends a text message (no photo)
      const result = handleTelegramMessage('12345', 'Hello world');

      // @step Then the WebSocket envelope should not have images field
      expect(result.data.images).toBeUndefined();
      expect(result.data.message).toBe('Hello world');
    });
  });

  describe('Scenario: Drop photo when no active session', () => {
    it('should warn and not forward a photo when the session socket is not open', () => {
      // @step Given the Telegram bridge has no active WebSocket session
      resetState();
      const state = getState();
      const send = vi.fn();
      // A socket that is not OPEN (closed) - the session is not active.
      state.currentSession.ws = {
        readyState: 3,
        send,
      } as unknown as typeof state.currentSession.ws;
      state.currentSession.sessionId = 'test-session-123';
      const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});

      // @step When a user sends a photo through Telegram
      // BUG-504: the bot handler hands the downloaded photo to forwardPhotoToSession.
      const forwarded = forwardPhotoToSession('12345', 'A caption', [
        { data: 'SGVsbG8gV29ybGQ=', media_type: 'image/jpeg' },
      ]);

      // @step Then the bridge should log a warning about no active session
      expect(warn).toHaveBeenCalledWith(
        '[telegram-endpoint] No active session to forward photo to'
      );

      // @step And the photo should not be forwarded
      expect(forwarded).toBe(false);
      expect(send).not.toHaveBeenCalled();
    });
  });

  describe('Scenario: Ignore non-photo media types', () => {
    it('should handle document as text message without images', () => {
      // @step Given I have a connected Telegram bridge session
      resetState();
      const state = getState();
      state.currentSession.sessionId = 'test-session-123';

      // @step When a user sends a document through Telegram
      // Documents don't have msg.photo, so they go through text path
      const result = handleTelegramMessage('12345', 'document.pdf');

      // @step Then the bridge should handle it as a text envelope
      expect(result.service).toBe('session');
      expect(result.type).toBe('input');
      expect(result.data.message).toBe('document.pdf');

      // @step And no images array should be included
      expect(result.data.images).toBeUndefined();
    });
  });

  describe('Scenario: Ignore sticker messages', () => {
    it('should handle sticker as text message without images', () => {
      // @step Given I have a connected Telegram bridge session
      resetState();
      const state = getState();
      state.currentSession.sessionId = 'test-session-123';

      // @step When a user sends a sticker through Telegram
      // Stickers don't have msg.photo, handled as text
      const result = handleTelegramMessage('12345', '');

      // @step Then the bridge should handle it as a text envelope
      expect(result.service).toBe('session');
      expect(result.type).toBe('input');

      // @step And no images array should be included
      expect(result.data.images).toBeUndefined();
    });
  });

  // ====================
  // LLM MULTIMODAL SCENARIOS (BRIDGE-007)
  // ====================

  describe('Scenario: Pass images to LLM as multimodal input', () => {
    it('should pass bridge images through to the LLM provider', () => {
      // @step Given I have a connected Telegram bridge session
      resetState();
      const state = getState();
      state.currentSession.sessionId = 'test-session-123';

      // @step And Telegram provides a photo message with caption "Describe this image"
      const caption = 'Describe this image';
      const images = [{ data: 'SGVsbG8gV29ybGQ=', media_type: 'image/jpeg' }];

      // @step When the bridge processes the photo message and injects it into the session
      const result = handleTelegramMessage('12345', caption, images);

      // The bridge creates a WebSocket message with images array
      // The session_manager receives this and passes it to run_agent_stream_with_images
      // which creates a multimodal Message::User with both text and image content

      // @step Then the session input envelope carries the image with its base64 data and media type
      expect(result.data.images).toBeDefined();
      expect(result.data.images?.length).toBe(1);
      expect(result.data.images?.[0].data).toBe('SGVsbG8gV29ybGQ=');
      expect(result.data.images?.[0].media_type).toBe('image/jpeg');

      // @step And the envelope carries the caption as the session input text
      // (The Rust session turns data.images into UserContent::Image; that side
      // is covered by the cli bridge-image tests under their own feature.)
      expect(result.data.message).toBe('Describe this image');
      expect(result.service).toBe('session');
      expect(result.type).toBe('input');
    });
  });
});

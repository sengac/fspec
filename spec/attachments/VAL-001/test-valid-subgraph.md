# Test Document with Valid Mermaid Diagram

This document contains a mermaid diagram with CORRECT subgraph syntax (no quotes).

## Audio Streaming Flow (Valid Syntax)

```mermaid
graph LR
    subgraph ServerSide[Server Side]
        AudioCapture[Audio Capture] --> Encoder[Opus Encoder]
        Encoder --> NetworkSend[Network Send]
    end

    subgraph ClientSide[Client Side]
        NetworkReceive[Network Receive] --> Decoder[Opus Decoder]
        Decoder --> AudioPlayback[Audio Playback]
    end

    NetworkSend --> NetworkReceive
```

This diagram uses `subgraph ServerSide[Server Side]` which is VALID and should be accepted.
